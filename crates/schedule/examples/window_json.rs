//! A window as the schedule reads it, for a client that has to write the same line: one candidate per line of stdin, one JSON line out each -
//! `{"ok":false}` for what is no window, else the days (a mask, Monday the lowest bit), the span, the profile (`null` for none) and `opens` - the minutes of the day at which the window begins to be open on some day
//! of the week, found by asking `Window::active` of every minute of the week: what the schedule itself says, for a client that has to tell a person when it will.
//!
//!     printf 'daily 22:00-06:00 eco\nmon 07:00+3h/20m off\n' | cargo run -q -p schedule --example window_json
//!
//! The page's form (page/form.js) builds and parses windows in JavaScript, and tools/test_page_window.js holds it to this reader on every kind.

use std::io::BufRead;

use schedule::{profile_word, Span, Wall, Window};

fn main() {
    for line in std::io::stdin().lock().lines() {
        let line = line.expect("a line of stdin");
        match Window::parse(&line) {
            Err(_) => println!("{{\"ok\":false}}"),
            Ok(w) => {
                let span = match w.span {
                    Span::Between { from, to } => format!("{{\"between\":[{from},{to}]}}"),
                    Span::Every { from, every_h, for_min } => format!("{{\"every\":[{from},{every_h},{for_min}]}}"),
                };
                let profile = w.profile.map_or("null".to_string(), |p| format!("\"{}\"", profile_word(p)));
                let open = |day: u8, minute: u16| w.active(Wall { day, minute }).is_some();
                let mut opens: Vec<u16> = (0..7u8)
                    .flat_map(|day| (0..1440u16).map(move |m| (day, m)))
                    .filter(|(day, m)| open(*day, *m) && !(if *m == 0 { open((*day + 6) % 7, 1439) } else { open(*day, m - 1) }))
                    .map(|(_, m)| m)
                    .collect();
                opens.sort_unstable();
                opens.dedup();
                let opens = opens.iter().map(u16::to_string).collect::<Vec<_>>().join(",");
                println!("{{\"ok\":true,\"days\":{},\"span\":{span},\"profile\":{profile},\"opens\":[{opens}]}}", w.days);
            }
        }
    }
}
