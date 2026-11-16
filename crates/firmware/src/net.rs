//! The station: joins one of the networks the recovery image saved, confirms the image once it has an
//! address, joins again whenever the link is lost, and moves to a stronger access point of the same network when its
//! own is weak.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use esp_idf_svc::eventloop::{EspSubscription, EspSystemEventLoop, System};
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_svc::ota::EspOta;
use esp_idf_svc::sys;
use esp_idf_svc::ipv4;
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDeviceId, WifiDriver, WifiEvent};

// Where the recovery image keeps the networks (scrivo-safe/main/main.c: NVS_NAMESPACE, WIFI_SLOTS,
// slot_keys): blobs wifi_ssid/wifi_pass for slot 0, wifi_ssid<n>/wifi_pass<n> for the others.
const NVS_NAMESPACE: &str = "scrivo";
const WIFI_SLOTS: usize = 5;

// The router has been seen refusing the first attempts for about 30 s (the recovery image waits as long).
const JOIN_TIMEOUT: Duration = Duration::from_secs(40);
const RETRY_PAUSE: Duration = Duration::from_secs(30);

/// How often a network that cannot be joined says so: once in five minutes, after the first line.
const NOT_JOINED_EVERY_S: u32 = 300;
const LINK_CHECK: Duration = Duration::from_secs(5);
// judged every 5 min, scanned for under -70 dBm, moved to at 10 dB stronger, three failed moves end it
const ROAM: roam::Rules = roam::Rules { weak_below: -70, every_s: 300.0, gain: 10, tries: 3 };
// a scan right before a connect made the router drop the association (scrivo-safe SCAN_CONNECT_GAP_MS)
const SCAN_CONNECT_GAP: Duration = Duration::from_secs(5);

struct Network {
    ssid: String,
    pass: String,
}

fn load_networks(nvs: &EspNvs<NvsDefault>) -> Vec<Network> {
    let mut nets = Vec::new();
    for slot in 0..WIFI_SLOTS {
        let (ssid_key, pass_key) = if slot == 0 {
            ("wifi_ssid".to_string(), "wifi_pass".to_string())
        } else {
            (format!("wifi_ssid{slot}"), format!("wifi_pass{slot}"))
        };
        let mut ssid_buf = [0u8; 33];
        let mut pass_buf = [0u8; 65];
        let ssid = match nvs.get_blob(&ssid_key, &mut ssid_buf) {
            Ok(Some(b)) if !b.is_empty() => String::from_utf8_lossy(b).into_owned(),
            _ => continue,
        };
        let pass = match nvs.get_blob(&pass_key, &mut pass_buf) {
            Ok(Some(b)) => String::from_utf8_lossy(b).into_owned(),
            other => {
                log::warn!("saved network '{ssid}' in slot {slot} has no usable {pass_key}: {other:?}");
                continue;
            }
        };
        log::info!("saved network {}: '{ssid}'", nets.len());
        nets.push(Network { ssid, pass });
    }
    nets
}

/// Joins NET: at the access point AT when it is given, else at the strongest of the network on any channel - the
/// default of `ClientConfiguration` scans every channel and sorts by signal.
fn join(wifi: &mut BlockingWifi<EspWifi<'static>>, net: &Network, at: Option<&roam::Found>) -> Result<(), sys::EspError> {
    let auth = if net.pass.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal };
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: net.ssid.as_str().try_into().unwrap_or_default(),
        password: net.pass.as_str().try_into().unwrap_or_default(),
        auth_method: auth,
        bssid: at.map(|f| f.bssid),
        channel: at.map(|f| f.channel),
        ..Default::default()
    }))?;
    wifi.wifi_mut().connect()?;
    wifi.wifi_wait_while(|| wifi.is_connected().map(|c| !c), Some(JOIN_TIMEOUT))?;
    wifi.ip_wait_while(|| wifi.is_up().map(|up| !up), Some(JOIN_TIMEOUT))
}

/// One look for a stronger access point of NET while joined at LINK, and the move to it when there is one. A move that
/// did not hold is followed by a join at the strongest of the network, so the station is not left out of it.
fn look_for_stronger(wifi: &mut BlockingWifi<EspWifi<'static>>, net: &Network, link: &crate::board::Link, roaming: &mut roam::Roam) {
    let found = match scan(wifi) {
        Ok(found) => found,
        Err(e) => return log::warn!("scan for a stronger access point failed: {e}"),
    };
    let Some(to) = roaming.choose(&link.ssid, link.bssid, link.rssi, &found, &ROAM) else {
        return log::info!(
            "'{}' at {} is weak ({} dBm) and no access point of it is {} dB stronger",
            net.ssid,
            crate::board::mac(&link.bssid),
            link.rssi,
            ROAM.gain
        );
    };
    log::info!(
        "moving from {} ({} dBm, channel {}) to {} ({} dBm, channel {})",
        crate::board::mac(&link.bssid),
        link.rssi,
        link.channel,
        crate::board::mac(&to.bssid),
        to.rssi,
        to.channel
    );
    std::thread::sleep(SCAN_CONNECT_GAP);
    let moved = wifi.disconnect().and_then(|_| join(wifi, net, Some(&to)));
    roaming.moved(moved.is_ok());
    match moved {
        Ok(()) => log::info!("moved to {}", crate::board::mac(&to.bssid)),
        Err(e) => {
            log::warn!("move to {} failed: {e} - joining the strongest of '{}' again", crate::board::mac(&to.bssid), net.ssid);
            let _ = wifi.wifi_mut().disconnect();
            if let Err(e) = join(wifi, net, None) {
                log::warn!("'{}' not joined again: {e}", net.ssid);
            }
        }
    }
    web::announce(page::SYSTEM);
}

fn confirm_image() {
    match EspOta::new().and_then(|mut ota| ota.mark_running_slot_valid()) {
        Ok(()) => log::info!("network up - image confirmed"),
        Err(e) => log::error!("image not confirmed: {e}"),
    }
}

pub struct Station {
    wifi: BlockingWifi<EspWifi<'static>>,
    nets: Vec<Network>,
    /// mDNS lives as long as the station: dropped, the board stops answering to its name.
    mdns: Option<EspMdns>,
    /// What the radio is made again from when it is reset: the device's name, the event loop, the partition.
    device: String,
    sysloop: EspSystemEventLoop,
    part: EspDefaultNvsPartition,
    _breaks: EspSubscription<'static, System>,
}

/// The station's network interface with the name the board gives itself in DHCP: the router takes it from the first request and lists it. esp-idf-svc sets a hostname only when it makes the interface, from
/// the configuration of its DHCP client, so the interface is made here and the Wi-Fi driver is wrapped around it. The name is None when the device's name is no host name, the MAC cannot be
/// read; the board then keeps the name IDF gives it, and the log says so.
fn station_netif(device: &str, driver: &WifiDriver<'static>) -> Result<(EspNetif, Option<String>), sys::EspError> {
    let name = match driver.get_mac(WifiDeviceId::Sta) {
        Ok(mac) => match board_name::of(device, mac) {
            Some(name) => Some(name),
            None => {
                log::warn!("name: the device's name '{device}' is no host name - the board keeps the name IDF gives it");
                None
            }
        },
        Err(e) => {
            log::warn!("name: the station's MAC not read ({e}) - the board keeps the name IDF gives it");
            None
        }
    };
    // `NAME_MAX` is the size of the client's hostname, so a name `board_name` gives always fits it
    let hostname = name.as_deref().and_then(|n| Hostname::try_from(n).ok());
    let dhcp = ipv4::ClientConfiguration::DHCP(ipv4::DHCPClientSettings { hostname: hostname.clone() });
    let netif = EspNetif::new_with_conf(&NetifConfiguration { ip_configuration: Some(ipv4::Configuration::Client(dhcp)), ..NetifConfiguration::wifi_default_client() })?;
    if let Some(hostname) = &hostname {
        log::info!("name: '{hostname}' in DHCP");
    }
    Ok((netif, hostname.map(|h| h.as_str().to_owned())))
}

/// The hostname of the DHCP client, as esp-idf-svc holds it, sized by the longest name the board gives itself: if esp-idf-svc's size and `board_name::NAME_MAX` part, this stops building.
type Hostname = heapless::String<{ board_name::NAME_MAX }>;

/// mDNS under the board's name: `<name>.local` answers with the board's address once it has one.
fn answer_to(name: &str) -> Option<EspMdns> {
    let mut mdns = match EspMdns::take() {
        Ok(mdns) => mdns,
        Err(e) => {
            log::warn!("mdns: not started: {e}");
            return None;
        }
    };
    if let Err(e) = mdns.set_hostname(name).and_then(|()| mdns.set_instance_name(name)) {
        log::warn!("mdns: '{name}' not set: {e}");
        return None;
    }
    log::info!("mdns: answering to '{name}.local'");
    Some(mdns)
}

/// The Wi-Fi driver and the network stack on it, with the board's name in DHCP and mDNS. Made at start and again when the radio is reset, which is why it is a function: the driver is made from nothing each time.
///
/// **THE DRIVER WRITES NO MORE OF ITS SETTINGS TO THE FLASH** (`WIFI_STORAGE_RAM`): its default is to keep the joined access point and the key there. The mode stops later writes only: what the NVS already holds is not erased, and
/// `WifiDriver::new` (the radio's init) reads it before the mode is set. So the sheet is not clean at start; the networks the board joins come from our own store (`load_networks`), set in the driver before every join.
fn bring_up(
    device: &str,
    modem: Modem<'static>,
    sysloop: &EspSystemEventLoop,
    part: &EspDefaultNvsPartition,
) -> Result<(BlockingWifi<EspWifi<'static>>, Option<EspMdns>), sys::EspError> {
    let driver = WifiDriver::new(modem, sysloop.clone(), Some(part.clone()))?;
    sys::esp!(unsafe { sys::esp_wifi_set_storage(sys::wifi_storage_t_WIFI_STORAGE_RAM) })?;
    let (sta, name) = station_netif(device, &driver)?;
    let mut wifi = BlockingWifi::wrap(EspWifi::wrap_all(driver, sta, EspNetif::new(NetifStack::Ap)?)?, sysloop.clone())?;
    wifi.start()?;
    // Radio sleep broke the handshake with our router on this board (docs/REF_Wifi_Codes.md).
    unsafe { sys::esp_wifi_set_ps(sys::wifi_ps_type_t_WIFI_PS_NONE) };
    let mdns = name.as_deref().and_then(answer_to);
    Ok((wifi, mdns))
}

/// Sets the Wi-Fi driver up, and with it the network stack: nothing may open a socket before this. DEVICE is the device's name from its description: the board is named
/// after it in DHCP and mDNS (`<device>-f950`) before the first join, the name being the station's to set.
pub fn start(device: &str, modem: Modem<'static>, sysloop: EspSystemEventLoop, part: EspDefaultNvsPartition) -> Result<Station, sys::EspError> {
    let nets = match EspNvs::new(part.clone(), NVS_NAMESPACE, false) {
        Ok(nvs) => load_networks(&nvs),
        Err(e) => {
            log::warn!("no saved networks: namespace '{NVS_NAMESPACE}': {e}");
            Vec::new()
        }
    };
    let (wifi, mdns) = bring_up(device, modem, &sysloop, &part)?;
    let breaks = watch_breaks(&sysloop)?;
    Ok(Station { wifi, nets, mdns, device: device.to_owned(), sysloop, part, _breaks: breaks })
}

/// The radio made again from nothing: mDNS and the driver with its network interface dropped - the driver's drop deinitialises the radio -, then made as at start. The modem is free once the driver is gone.
fn reset_radio(
    wifi: BlockingWifi<EspWifi<'static>>,
    mdns: Option<EspMdns>,
    device: &str,
    sysloop: &EspSystemEventLoop,
    part: &EspDefaultNvsPartition,
) -> Result<(BlockingWifi<EspWifi<'static>>, Option<EspMdns>), sys::EspError> {
    drop(mdns);
    drop(wifi);
    bring_up(device, unsafe { Modem::steal() }, sysloop, part)
}

/// The board restarts: the line goes into the journal first, and the pause gives it the time to leave.
fn restart(why: &str) -> ! {
    log::error!("{why}");
    std::thread::sleep(Duration::from_millis(1000));
    unsafe { sys::esp_restart() };
    #[allow(unreachable_code)]
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

// ---- a handshake that breaks (ESP-IDF #18061) ----

static BREAKS_FROM: OnceLock<between::clock::Moment> = OnceLock::new();
/// The handshakes that broke, counted for each access point by its BSSID: a line the first time and once in five minutes after it, with how many there were at that point. A join ends the run of the point it was at only.
static BREAKS: Mutex<journal::Keyed<[u8; 6], BREAKS_KEPT>> = Mutex::new(journal::Keyed::new(NOT_JOINED_EVERY_S));
/// How many access points have their count of breaks at once: the strongest of one network and the next ones of it, with room over.
const BREAKS_KEPT: usize = 8;
/// The channel of the access point the last join was asked for; 0 when the driver chose.
static TRIED_CHANNEL: AtomicU8 = AtomicU8::new(0);

/// Says what the driver says about a handshake that broke (reason 15): at which access point, on which channel, at what signal, and which break of the run it is. The radio gives the reason; the channel is the one the join was asked for.
fn watch_breaks(sysloop: &EspSystemEventLoop) -> Result<EspSubscription<'static, System>, sys::EspError> {
    sysloop.subscribe::<WifiEvent, _>(|event| {
        let WifiEvent::StaDisconnected(d) = event else { return };
        if u32::from(d.reason()) != sys::wifi_err_reason_t_WIFI_REASON_4WAY_HANDSHAKE_TIMEOUT {
            return;
        }
        let now_s = between::clock::now().since(*BREAKS_FROM.get_or_init(between::clock::now)).as_secs();
        let Some(said) = BREAKS.lock().ok().and_then(|mut b| b.fail(d.bssid(), now_s)) else { return };
        let channel = match TRIED_CHANNEL.load(Ordering::Relaxed) {
            0 => String::new(),
            c => format!(", channel {c}"),
        };
        log::warn!(
            "handshake broke at {} (reason {}{channel}, {} dBm): break {}, {} since the last line unsaid",
            crate::board::mac(&d.bssid()),
            d.reason(),
            d.rssi(),
            said.attempts,
            said.unsaid
        );
    })
}

/// The numbers the station gets out of a network by, as they are set now.
fn rules() -> rescue::Rules {
    let now = crate::nvs_settings::now();
    let number = |s: &settings::Setting| now.value(s).parse().unwrap_or_else(|_| s.default.parse().unwrap_or(2));
    rescue::Rules {
        tries: number(&settings::NET_POINT_TRIES),
        reset_laps: number(&settings::NET_RESET_LAPS),
        reboot_laps: number(&settings::NET_REBOOT_LAPS),
    }
}

/// What a scan shows now.
fn scan(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<Vec<roam::Found>, sys::EspError> {
    Ok(wifi
        .scan()?
        .into_iter()
        .map(|a| roam::Found { ssid: a.ssid.to_string(), bssid: a.bssid, channel: a.channel, rssi: a.signal_strength })
        .collect())
}

/// Joins and keeps joining for good; returns only when there is no network to join.
///
/// Every try is at an access point chosen from a fresh scan (`rescue::Rescue`): one that failed twice in a row is left for the next strongest of the network, and the laps without a join end in the radio's reset and then in a restart of the board.
pub fn run(station: Station) {
    let Station { mut wifi, nets, mut mdns, device, sysloop, part, _breaks } = station;
    if nets.is_empty() {
        log::error!("no saved networks - the recovery image's page sets them; staying without network");
        return;
    }
    let mut confirmed = false;
    // SNTP's owner is this task: it knows the router's address, and a changed setting reaches it by a signal
    let mut sntp = crate::clock::Sntp::default();
    // A NETWORK THAT CANNOT BE JOINED FAILS EVERY ~74 S, AND THE FIRST LINE SAYS IT: the tries that follow are counted for each network, and the line comes again once in five minutes with how many there have been.
    let mut failing = journal::Each::new(NOT_JOINED_EVERY_S, nets.len());
    let started = between::clock::now();
    let mut rescue = rescue::Rescue::new();
    loop {
        for (place, net) in nets.iter().enumerate() {
            let rules = rules();
            let found = scan(&mut wifi).unwrap_or_else(|e| {
                log::debug!("scan before a join failed: {e}");
                Vec::new()
            });
            let at = match rescue.next(&net.ssid, &found, &rules) {
                rescue::Next::Try(point) => Some(point),
                rescue::Next::Any => None,
                rescue::Next::ResetRadio => {
                    log::warn!("no network in {} laps over '{}': the radio is reset", rules.reset_laps, net.ssid);
                    match reset_radio(wifi, mdns, &device, &sysloop, &part) {
                        Ok((w, m)) => (wifi, mdns) = (w, m),
                        Err(e) => restart(&format!("the radio did not come back after its reset ({e}): the board restarts")),
                    }
                    continue;
                }
                rescue::Next::Restart => {
                    restart(&format!("no network in {} laps after the radio's reset: the board restarts", rules.reboot_laps))
                }
            };
            if failing.quiet() {
                log::info!("joining '{}'", net.ssid);
            } else {
                log::debug!("joining '{}' again", net.ssid);
            }
            // a scan right before a connect made the router drop the association (SCAN_CONNECT_GAP)
            if at.is_some() {
                std::thread::sleep(SCAN_CONNECT_GAP);
            }
            TRIED_CHANNEL.store(at.as_ref().map_or(0, |p| p.channel), Ordering::Relaxed);
            match join(&mut wifi, net, at.as_ref()) {
                Ok(()) => {
                    failing.ok();
                    rescue.joined();
                    if let Ok(mut b) = BREAKS.lock() {
                        match crate::board::link() {
                            Some(link) => b.ok(&link.bssid),
                            None => b.ok_all(),
                        }
                    }
                    let info = wifi.wifi().sta_netif().get_ip_info().ok();
                    let ip = info.map(|i| i.ip.to_string()).unwrap_or_default();
                    log::info!("joined '{}', address {ip}", net.ssid);
                    sntp.start(info.map(|i| i.subnet.gateway.to_string()));
                    web::announce(page::SYSTEM);
                    if !confirmed && cfg!(feature = "never-confirm") {
                        log::warn!("built never to confirm itself: the next reset returns the board to factory");
                        confirmed = true;
                    } else if !confirmed {
                        confirm_image();
                        confirmed = true;
                    }
                    let mut roaming = roam::Roam::new();
                    let mut was = between::clock::now();
                    while wifi.is_connected().unwrap_or(false) {
                        sntp.wait(LINK_CHECK);
                        let now = between::clock::now();
                        let secs = now.since(was).as_secs_f32();
                        was = now;
                        let Some(link) = crate::board::link() else { continue };
                        if roaming.second(link.rssi, secs, &ROAM) {
                            look_for_stronger(&mut wifi, net, &link, &mut roaming);
                        }
                    }
                    log::warn!("link to '{}' lost", net.ssid);
                    web::announce(page::SYSTEM);
                }
                Err(e) => {
                    if let Some(said) = failing.fail(place, between::clock::now().since(started).as_secs()) {
                        log::warn!("'{}' not joined: {e} (try {}, {} since the last line unsaid)", net.ssid, said.attempts, said.unsaid);
                    }
                    if let (true, Some(point)) = (rescue.failed(at.as_ref().map(|p| p.bssid), &rules), &at) {
                        log::warn!(
                            "access point {} of '{}' (channel {}, {} dBm) left after {} failed joins in a row: the next try goes to the next strongest",
                            crate::board::mac(&point.bssid),
                            net.ssid,
                            point.channel,
                            point.rssi,
                            rules.tries
                        );
                    }
                    let _ = wifi.wifi_mut().disconnect();
                }
            }
        }
        std::thread::sleep(RETRY_PAUSE);
    }
}
