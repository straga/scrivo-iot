//! The values a stand of a device starts from: a made-up board for the page (`tools/page_stand.py`). The board's own card as a board that has joined its network has it, every setting at its default as the
//! text it is kept as, and the device's own values on top, written by the same functions the board writes them with - so the stand says nothing the dictionary does not have.

use page::dictionary::{Entry, Kind, Said, Value};

/// The stand's state for TABLE of the dictionary VERSION, every setting of SETTINGS at its default, as the JSON the stand reads: `contract`, `dictionary` and `values`, [address, value] each in the order of the table. DEVICE puts the device's own values.
pub fn state_json(table: &[Entry], settings: &[&settings::Setting], version: &str, device: &dyn Fn(&mut dyn FnMut(usize, Said<'_>))) -> String {
    let mut values: Vec<(usize, serde_json::Value)> = Vec::new();
    let mut put = |at: usize, said: Said<'_>| {
        let Some(entry) = table.get(at) else { return };
        if entry.fixed.is_some() || entry.kinds.contains(&Kind::Button) {
            return;
        }
        let value = serde_json::to_value(said).expect("a value is JSON");
        match values.iter_mut().find(|(a, _)| *a == at) {
            Some(held) => held.1 = value,
            None => values.push((at, value)),
        }
    };
    system(&mut put);
    // the rows of the board's card a device puts last (`system::CORES`), when it has them: a core a fifth busy and one hardly at all
    for (name, share) in [("core_0", 21.0), ("core_1", 4.0)] {
        if let Some(at) = page::dictionary::address(table, page::SYSTEM, name) {
            put(at, Said::Value(Value::Number(share)));
        }
    }
    for (at, entry) in table.iter().enumerate().filter(|(_, e)| e.card == page::SETTINGS && e.part_of.is_none()) {
        let default = settings.iter().find(|s| s.name == entry.name).map_or("", |s| s.default);
        put(at, Said::Text(default));
    }
    device(&mut put);
    values.sort_by_key(|(at, _)| *at);
    serde_json::json!({
        "contract": page::dictionary::CONTRACT,
        "dictionary": version,
        "values": values.into_iter().map(|(at, v)| serde_json::json!([at, v])).collect::<Vec<_>>(),
    })
    .to_string()
}

/// The board's own card of a board that has joined its network, a few minutes up, with its memory in order.
fn system(put: &mut dyn FnMut(usize, Said<'_>)) {
    use crate::system as at;
    put(at::CONSOLE, Said::Value(Value::Number(0.0)));
    put(at::WIFI, Said::Value(Value::Word("joined")));
    put(at::WIFI_IP, Said::Text("192.0.2.40"));
    put(at::WIFI_SSID, Said::Text("GZ103"));
    put(at::WIFI_SIGNAL, Said::Value(Value::Number(-58.0)));
    put(at::FREE, Said::Value(Value::Number(110_592.0)));
    put(at::USED, Said::Value(Value::Number(214_016.0)));
    put(at::LOW_MEMORY, Said::Value(Value::On(false)));
    put(at::UPTIME, Said::Value(Value::Number(13_780_000.0)));
    put(at::TIME, Said::Text("2026-10-08 17:40"));
    put(at::TIME_ZONE, Said::Text("EEST"));
    put(at::TIME_FROM, Said::Value(Value::Word("gateway")));
    put(at::TIME_GATEWAY, Said::Text("192.0.2.1"));
}
