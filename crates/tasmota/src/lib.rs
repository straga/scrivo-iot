//! A Tasmota socket's energy status read over HTTP: the request the board may send it, the answer taken apart into watts, volts and amps, and the exchange over a TCP socket within a time.
//! HERE: what is sent, what comes back and what it says. The one request is a READ: the socket powers the whole burner, and a command to it is never built in this crate.
//! NEVER HERE: when to ask (the module), what the number is called or shown as (api/, page/menu.json), a connection of the board's own (the connector comes in as a parameter, so the tests
//! run this over the computer's sockets and the board over lwIP's).
//! CHECKS: tests/parse.rs, tests/exchange.rs, tests/over_tcp.rs.
//!
//! THE ANSWER IS ASKED IN HTTP/1.0 ON PURPOSE: a socket answers a 1.1 request in chunks (`Transfer-Encoding: chunked`, sizes between the pieces of the body) and a 1.0 request with the body whole, ended by the
//! socket closing the connection [probed on a Tasmota 8.4.0 socket]. The board reads to that close, so the closing is the socket's: the connection waits out its two minutes on the socket's side and not on the board's.

use core::net::SocketAddr;
use core::time::Duration;

use edge_nal::TcpConnect;
use embedded_io_async::{Read, Write};
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// The only thing this crate ever sends a socket: a read of its sensor status, which carries the `ENERGY` block. Nothing in the request changes the socket's state.
pub const REQUEST: &[u8] = b"GET /cm?cmnd=Status%2010 HTTP/1.0\r\nHost: board\r\nConnection: close\r\n\r\n";

/// What a socket measures: the power in watts, the voltage in volts, the current in amps.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Energy {
    pub watts: f32,
    pub volts: f32,
    pub amps: f32,
}

/// Why there is no reading.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fault {
    /// Nobody answered within the time, or the connection broke while it was answering.
    Silent,
    /// The connection was not made: nobody listens at the address, or there is no way to it.
    Unreachable,
    /// The connection closed and said nothing.
    Closed,
    /// More came than the buffer holds.
    TooLong,
    /// Somebody answered with a status other than 200.
    Refused(u16),
    /// The answer is in chunks, which this crate does not read: the sizes between the pieces would be taken for the body.
    Chunked,
    /// The head or the body is not what a socket says.
    NotJson,
    /// A socket answered, and it has no meter: its status carries no `ENERGY`, or lacks a quantity of it, or the quantity is not a number.
    NoEnergy,
}

impl Fault {
    /// The code a client is told for this fault: `silent` - nobody is there to ask -, or `unreadable` - somebody is there and it is not a meter this crate can read.
    pub fn reason(self) -> &'static str {
        match self {
            Fault::Silent | Fault::Unreachable | Fault::Closed => "silent",
            Fault::TooLong | Fault::Refused(_) | Fault::Chunked | Fault::NotJson | Fault::NoEnergy => "unreadable",
        }
    }
}

impl core::fmt::Display for Fault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Fault::Silent => f.write_str("no answer in time"),
            Fault::Unreachable => f.write_str("no connection"),
            Fault::Closed => f.write_str("closed without an answer"),
            Fault::TooLong => f.write_str("the answer is longer than the buffer"),
            Fault::Refused(status) => write!(f, "status {status}"),
            Fault::Chunked => f.write_str("the answer is in chunks"),
            Fault::NotJson => f.write_str("the answer is not a socket's"),
            Fault::NoEnergy => f.write_str("the socket has no meter"),
        }
    }
}

/// The body of an HTTP answer that says 200 and is not in chunks.
pub fn body_of(answer: &[u8]) -> Result<&[u8], Fault> {
    let end = answer.windows(4).position(|w| w == b"\r\n\r\n").ok_or(Fault::NotJson)?;
    let (head, body) = (&answer[..end], &answer[end + 4..]);
    let line = head.split(|b| *b == b'\r').next().unwrap_or(head);
    if !line.starts_with(b"HTTP/1.") || line.len() < 12 || line[8] != b' ' {
        return Err(Fault::NotJson);
    }
    let status = core::str::from_utf8(&line[9..12]).ok().and_then(|s| s.parse::<u16>().ok()).ok_or(Fault::NotJson)?;
    if status != 200 {
        return Err(Fault::Refused(status));
    }
    if head.windows(26).any(|w| w.eq_ignore_ascii_case(b"transfer-encoding: chunked")) {
        return Err(Fault::Chunked);
    }
    Ok(body)
}

/// A quantity as a number, or None for anything else written in its place.
fn number<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    struct Number;
    impl<'de> Visitor<'de> for Number {
        type Value = Option<f64>;
        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("a number or anything else")
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }
        fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    d.deserialize_any(Number)
}

#[derive(Deserialize)]
struct Status {
    #[serde(rename = "StatusSNS")]
    sensors: Option<Sensors>,
}

#[derive(Deserialize)]
struct Sensors {
    #[serde(rename = "ENERGY")]
    energy: Option<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "Power", default, deserialize_with = "number")]
    power: Option<f64>,
    #[serde(rename = "Voltage", default, deserialize_with = "number")]
    voltage: Option<f64>,
    #[serde(rename = "Current", default, deserialize_with = "number")]
    current: Option<f64>,
}

/// The watts, volts and amps in the body of a `Status 10` answer.
pub fn parse(body: &[u8]) -> Result<Energy, Fault> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Err(Fault::Closed);
    }
    let status: Status = serde_json::from_slice(body).map_err(|_| Fault::NotJson)?;
    let block = status.sensors.and_then(|s| s.energy).ok_or(Fault::NoEnergy)?;
    match (block.power, block.voltage, block.current) {
        (Some(watts), Some(volts), Some(amps)) => Ok(Energy { watts: watts as f32, volts: volts as f32, amps: amps as f32 }),
        _ => Err(Fault::NoEnergy),
    }
}

/// Sends [`REQUEST`] over SOCKET and reads the answer into BUF to the socket's close; the length of the answer.
pub async fn exchange<S: Read + Write>(socket: &mut S, buf: &mut [u8]) -> Result<usize, Fault> {
    socket.write_all(REQUEST).await.map_err(|_| Fault::Silent)?;
    let mut n = 0;
    while n < buf.len() {
        match socket.read(&mut buf[n..]).await {
            Ok(0) => break,
            Ok(more) => n += more,
            Err(_) => return Err(Fault::Silent),
        }
    }
    if n == buf.len() {
        // full: whole only if the socket has nothing more to say
        let mut more = [0u8; 1];
        match socket.read(&mut more).await {
            Ok(0) => {}
            Ok(_) => return Err(Fault::TooLong),
            Err(_) => return Err(Fault::Silent),
        }
    }
    if n == 0 {
        return Err(Fault::Closed);
    }
    Ok(n)
}

/// One reading of the socket at AT over NET: the connect, the request, the answer to the close and its taking apart, all within WITHIN. The connection is let go when this returns, the socket having closed first.
pub async fn read<C: TcpConnect>(net: &C, at: SocketAddr, buf: &mut [u8], within: Duration) -> Result<Energy, Fault> {
    let whole = async {
        let mut socket = net.connect(at).await.map_err(|_| Fault::Unreachable)?;
        exchange(&mut socket, buf).await
    };
    let n = embassy_time::with_timeout(embassy_time::Duration::from_micros(within.as_micros() as u64), whole).await.map_err(|_| Fault::Silent)??;
    parse(body_of(&buf[..n])?)
}
