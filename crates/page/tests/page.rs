//! The page's contract that is not about a value: which file answers for a name and in what coding, the answers of /restart and /down, how a name after a prefix is read, the tab's one save, the CSV of a month,
//! the order the paths are registered in, a curve and how it goes out. What a board says of its things is the dictionary's (`tests/dictionary.rs`, `tests/values.rs`, `tests/stream*.rs`).

use page::dictionary::{Entry, Kind};
use page::{Route, Way, BUSY};
use serde_json::json;

/// The table of a board with the settings NAMES: a row of the settings card each.
fn settings_of(names: &[&'static str]) -> &'static [Entry] {
    Box::leak(names.iter().map(|name| Entry::row(page::SETTINGS, name, &[Kind::Text])).collect::<Vec<_>>().into_boxed_slice())
}


#[test]
fn a_browser_that_takes_gzip_gets_the_gzipped_file() {
    assert_eq!(page::served("/page", "index.html", "gzip, deflate, br"), ("/page/index.html.gz".into(), true));
    assert_eq!(page::served("/page", "index.html", "br;q=1.0, gzip;q=0.8, *;q=0.1"), ("/page/index.html.gz".into(), true));
    assert_eq!(page::served("/page", "icon.svg", "gzip"), ("/page/icon.svg.gz".into(), true));
}

#[test]
fn a_client_that_takes_no_gzip_gets_the_plain_file_and_not_a_refusal() {
    for accept in ["", "identity", "deflate, br", "gzip;q=0, identity", "gzip;q=0.0"] {
        assert_eq!(page::served("/page", "index.html", accept), ("/page/index.html".into(), false), "{accept}");
    }
}

#[test]
fn down_with_the_board_s_name_goes() {
    assert_eq!(page::down("STRAGA_BASE_S3", "STRAGA_BASE_S3"), Ok(json!({"going": "recovery"})));
}

#[test]
fn down_with_another_name_or_none_is_refused_in_words() {
    for said in ["SONOFF_DUAL_R3", "", "straga_base_s3"] {
        let answer = page::down("STRAGA_BASE_S3", said).unwrap_err();
        assert_eq!(
            answer["error"],
            format!("the board goes down only when asked by its name, STRAGA_BASE_S3; the request said '{said}'")
        );
    }
}

#[test]
fn restart_says_how_long_the_page_is_without_the_board() {
    assert_eq!(page::restart(), json!({"going": "restart", "back_in_s": 20}));
}

#[test]
fn the_name_after_the_prefix_is_read_as_encodeuricomponent_wrote_it() {
    assert_eq!(page::name_in("/down/STRAGA_BASE_S3", "/down/"), "STRAGA_BASE_S3");
    assert_eq!(page::name_in("/down/a%20b", "/down/"), "a b");
    assert_eq!(page::name_in("/toggle/relay?x=1", "/toggle/"), "relay");
}

#[test]
fn a_broken_escape_is_kept_as_written_and_then_refused_as_a_name() {
    assert_eq!(page::name_in("/down/a%2", "/down/"), "a%2");
    assert_eq!(page::name_in("/down/%zz", "/down/"), "%zz");
    assert_eq!(page::name_in("/other/x", "/down/"), "");
}

#[test]
fn a_save_hands_the_board_every_pair_of_the_form_and_answers_what_changed_by_name_and_value() {
    let answer = page::save(settings_of(&["zone", "time"]), "zone=utc&time=time.google.com", |pairs| {
        assert_eq!(pairs, [("zone".to_string(), "utc".to_string()), ("time".to_string(), "time.google.com".to_string())]);
        Ok(vec![json!({"name": "zone", "value": "utc"})])
    });
    assert_eq!(answer, json!({"changed": [{"name": "zone", "value": "utc"}]}));
}

#[test]
fn a_form_is_read_as_a_browser_writes_it() {
    let answer = page::save(settings_of(&["time"]), "time=my+server%2Elan", |pairs| Ok(vec![json!({"name": "time", "value": pairs[0].1})]));
    assert_eq!(answer, json!({"changed": [{"name": "time", "value": "my server.lan"}]}));
}

#[test]
fn a_save_the_board_refuses_answers_why_by_the_code_the_board_gave() {
    let answer = page::save(settings_of(&["zone"]), "zone=mars", |_| Err(json!({"code": "none_of", "name": "zone", "text": "mars", "choices": ["kyiv", "utc"]})));
    assert_eq!(answer, json!({"error": {"code": "none_of", "name": "zone", "text": "mars", "choices": ["kyiv", "utc"]}}));
}

#[test]
fn a_save_naming_a_setting_there_is_not_or_a_value_too_long_does_not_reach_the_board() {
    let never = |_: &[(String, String)]| -> Result<Vec<serde_json::Value>, serde_json::Value> { panic!("the board was asked") };
    assert_eq!(page::save(settings_of(&["zone"]), "wifi=x", never), json!({"error": {"code": "no_setting", "name": "wifi"}}));
    assert_eq!(page::save(settings_of(&["zone"]), "", never), json!({"error": {"code": "nothing_to_save"}}));
    let long = format!("zone={}", "x".repeat(page::MAX_VALUE + 1));
    assert_eq!(
        page::save(settings_of(&["zone"]), &long, never),
        json!({"error": {"code": "value_too_long", "name": "zone", "max": page::MAX_VALUE, "got": page::MAX_VALUE + 1}})
    );
}

#[test]
fn a_csv_of_a_month_is_served_from_the_board_s_data() {
    assert_eq!(page::csv_file("2026-09.csv"), Ok("/data/energy/2026-09.csv".to_string()));
}

#[test]
fn a_name_that_is_not_a_month_s_csv_reaches_no_file() {
    for bad in ["", "state", "../page/index.html", "2026-09.csv/..", "2026-9.csv", "2026-09.txt", "x2026-09.csv", "2026-13.csv", "%2e%2e"] {
        assert!(page::csv_file(bad).is_err(), "{bad} taken");
    }
}

#[test]
fn a_file_of_the_page_is_named_by_what_it_is() {
    // the browser refuses a stylesheet served as text/plain and will not run a script served as anything else
    assert_eq!(page::mime("app.js"), Some("text/javascript; charset=utf-8"));
    assert_eq!(page::mime("owl.iife.min.js"), Some("text/javascript; charset=utf-8"));
    assert_eq!(page::mime("app.tw.css"), Some("text/css; charset=utf-8"));
    assert_eq!(page::mime("index.html"), Some("text/html; charset=utf-8"));
    assert_eq!(page::mime("icon.svg"), Some("image/svg+xml"));
}

#[test]
fn a_name_that_is_not_a_file_of_a_page_is_refused() {
    // the page folder is what a browser may ask for, and nothing above or beside it
    for bad in ["", ".", "..", "../secrets", "sub/app.js", "/etc/passwd", "app.js/..", "app", "app.exe", "app.js.bak"] {
        assert_eq!(page::mime(bad), None, "{bad:?} was served");
    }
}

#[test]
fn the_gz_of_a_file_is_named_by_what_the_file_is() {
    // a gzipped script is still a script: the encoding is a header, not a type
    assert_eq!(page::mime("app.js.gz"), Some("text/javascript; charset=utf-8"));
    assert_eq!(page::mime("app.tw.css.gz"), Some("text/css; charset=utf-8"));
}

#[test]
fn a_wildcard_registered_first_swallows_the_console_and_the_stream() {
    // The server looks for an installed handler matching the new uri and method before installing one, and answers
    // ESP_ERR_HTTPD_HANDLER_EXISTS when it finds one (httpd_uri.c, httpd_register_uri_handler). /* answers for
    // /console, so /console is never installed, the server does not come up, and the image rolls back.
    let refused = page::refused(&the_order_that_rolled_four_images_back());
    assert!(refused.contains(&(Route::Console, Route::Rest)), "the console was let through: {refused:?}");
    assert!(refused.contains(&(Route::Events, Route::Rest)), "the stream was let through: {refused:?}");
}

#[test]
fn the_order_the_board_registers_lets_every_route_in() {
    // This is the guard on ORDER itself: move Route::Rest up and this fails, naming what it swallowed.
    assert_eq!(page::refused(&page::ORDER), vec![]);
}

#[test]
fn a_named_path_after_the_wildcard_is_refused_whichever_it_is() {
    // Not only the console: any GET path put after /* is swallowed the same way - one case per named GET route.
    for named in Route::ALL.iter().copied().filter(|r| *r != Route::Rest && r.way() == Way::Get) {
        let order = [Route::Rest, named];
        assert_eq!(page::refused(&order), vec![(named, Route::Rest)], "{} was let through after /*", named.uri());
    }
}

#[test]
fn a_post_path_is_not_swallowed_by_a_get_wildcard() {
    // The method is half the match: /* is a GET, so /restart POST goes in even behind it. The order still puts it
    // first - a rule nobody has to remember beats one they do - but the proof must not claim more than IDF does.
    for post in Route::ALL.iter().copied().filter(|r| r.way() == Way::Post) {
        assert_eq!(page::refused(&[Route::Rest, post]), vec![], "{} refused by a GET wildcard", post.uri());
    }
}

#[test]
fn every_route_is_registered_exactly_once() {
    // A different breakage from the order: a path that is not in ORDER at all is never served, and the page asking
    // for it gets a 404 that looks like a missing file.
    for route in Route::ALL {
        let times = page::ORDER.iter().filter(|r| **r == route).count();
        assert_eq!(times, 1, "{} is registered {} times", route.uri(), times);
    }
}

#[test]
fn a_template_answers_for_the_names_under_it_and_no_others() {
    // IDF's httpd_uri_match_wildcard with uri_match_wildcard on: a trailing * matches any tail, a template without
    // one matches its own name exactly.
    assert!(page::wildcard_match("/*", "/console"));
    assert!(page::wildcard_match("/*", "/"));
    assert!(page::wildcard_match("/*", "/app.js"));
    assert!(page::wildcard_match("/energy/*", "/energy/2026-09.csv"));
    assert!(page::wildcard_match("/state", "/state"));
    assert!(!page::wildcard_match("/energy/*", "/events"));
    assert!(!page::wildcard_match("/state", "/states"));
    assert!(!page::wildcard_match("/state", "/stat"));
    assert!(!page::wildcard_match("/icon.svg", "/favicon.ico"));
}

#[test]
fn no_route_uses_the_question_mark_our_matcher_does_not_model() {
    // IDF's matcher also knows `?`, standing for one optional character. Ours does not, so a template using it
    // would be judged by a matcher that is not the server's - and the proof would be about nothing.
    for route in Route::ALL {
        assert!(!route.uri().contains('?'), "{} uses ? which wildcard_match does not model", route.uri());
    }
}

#[test]
fn the_journal_kept_on_flash_is_read_by_a_get_of_its_own_path() {
    // what the burner did in the last run is read over HTTP, and the page is not needed
    let journal = Route::ALL.iter().copied().find(|r| r.uri() == "/journal").expect("no path reads the journal");
    assert_eq!(journal.way(), Way::Get);
    assert_eq!(page::ORDER.iter().filter(|r| **r == journal).count(), 1, "the journal is not registered once");
    assert!(page::wildcard_match(journal.uri(), "/journal"));
    // the pair: its path answers for itself and not for a name under it
    assert!(!page::wildcard_match(journal.uri(), "/journal/1.log"));
}

#[test]
fn a_curve_of_six_hundred_points_stays_within_five_kilobytes() {
    // THE NUMBER THIS IS JUDGED BY, and it is about memory rather than bytes on the wire: the answer has to
    // stand somewhere whole-ish while it is built, and the heap of an S3 with a page open has tens of
    // kilobytes in one piece, not hundreds. Six hundred points came to 12841 bytes on the board on 21.09,
    // because the answer went through serde_json::to_value and every f32 was widened to an f64 there -
    // 161.89999389648438 where the data says 161.9. Serialised as itself it is about a quarter of that.
    let points: Vec<f32> = (0..600).map(|i| 160.0 + (i % 7) as f32 / 10.0).collect();
    let said = serde_json::to_string(&curve_of(points)).expect("a curve serialises");
    assert!(said.len() < 5_000, "600 points came to {} bytes: {}", said.len(), &said[..120]);
    // and the widened form is not in it, which is what keeps the number above true
    assert!(!said.contains("160.09999"), "{}", &said[..200]);
}

#[test]
fn an_answer_goes_out_in_pieces_and_never_stands_whole() {
    // What the fall of 21.09 was: /curve built the whole answer in memory before a byte of it went out, and
    // the piece it asked the heap for was bigger than the biggest piece the heap had. Pieces is the shape that
    // makes the cost a fixed buffer instead of the size of the answer.
    let points: Vec<f32> = (0..600).map(|i| 160.0 + (i % 7) as f32 / 10.0).collect();
    let curve = curve_of(points);
    let whole = serde_json::to_string(&curve).expect("a curve serialises");

    let mut sent: Vec<Vec<u8>> = Vec::new();
    let mut pieces = page::Pieces::new(|piece: &[u8]| {
        sent.push(piece.to_vec());
        true
    });
    serde_json::to_writer(&mut pieces, &curve).expect("a curve serialises into pieces");
    assert!(pieces.done(), "every piece was taken");

    // the bytes are the same bytes - the shape of the answer is not what changed
    assert_eq!(String::from_utf8(sent.concat()).unwrap(), whole);
    // and not one of them is the whole answer
    assert!(sent.len() > 1, "{} pieces for {} bytes", sent.len(), whole.len());
    assert!(sent.iter().all(|piece| piece.len() <= page::PIECE), "a piece went over {}", page::PIECE);
}

#[test]
fn a_piece_the_other_end_refuses_stops_the_answer_instead_of_being_dropped() {
    // The pair. A browser that goes away mid-answer, or a socket that fills: carrying on writing into
    // nothing would leave the request looking answered, and the next piece costs another copy for nothing.
    let mut taken = 0;
    let mut pieces = page::Pieces::new(|_piece: &[u8]| {
        taken += 1;
        taken < 2
    });
    let points: Vec<f32> = (0..600).map(|i| i as f32).collect();
    let said = serde_json::to_writer(&mut pieces, &curve_of(points));
    assert!(said.is_err(), "a refused piece came back as {said:?}");
    assert!(!pieces.done(), "done() says the answer got through");
    assert_eq!(taken, 2, "pieces were still handed over after a refusal");
}

#[test]
fn a_curve_carries_as_many_lines_as_the_picture_needs() {
    // THREE ON ONE FIELD is what a regulator wants: what was asked, what came out, and the angle it is held by.
    // Without the third, "the regulator is twitching" and "the air is breathing" look the same on a picture.
    let line = |words: &str, unit: &str, points: Vec<f32>| page::Line {
        words: words.into(),
        unit: unit.into(),
        points,
        span: Some((0.0, 300.0)),
    };
    let drawn = page::Curve {
        step_ms: 1000,
        lines: vec![
            line("asked", "Pa", vec![51.0, 51.0]),
            line("came out", "Pa", vec![12.0, 31.0]),
            line("by angle", "%", vec![33.0, 18.0]),
        ],
    };
    let said: serde_json::Value = serde_json::to_value(&drawn).expect("a curve writes");
    assert_eq!(said["lines"].as_array().unwrap().len(), 3);
    assert_eq!(said["lines"][0]["words"], "asked");
    assert_eq!(said["lines"][2]["unit"], "%", "the angle is drawn in its own unit, not in pascals");
    // LINES THAT SHARE A UNIT SHARE A SCALE, and that is why there is no flag saying so: the first two are
    // pascals and the third is percent, and a picture cannot hold both on one axis
    assert_eq!(said["lines"][0]["unit"], said["lines"][1]["unit"]);
    assert_ne!(said["lines"][0]["unit"], said["lines"][2]["unit"]);
}

#[test]
fn a_line_without_a_span_carries_no_span_field_at_all() {
    // the page offers the button that puts a curve back against what the value CAN be only for a line that
    // says what that is; a line with no such limit must not come out as one with a span of null
    let drawn = page::Curve {
        step_ms: 1000,
        lines: vec![page::Line { words: "counted".into(), unit: "".into(), points: vec![1.0], span: None }],
    };
    let said: serde_json::Value = serde_json::to_value(&drawn).unwrap();
    assert!(said["lines"][0].get("span").is_none(), "a line with no span sent one: {said}");
}

#[test]
fn on_off_and_press_are_routes_a_page_posts_to() {
    for (route, uri) in [(Route::On, "/on/*"), (Route::Off, "/off/*"), (Route::Press, "/press/*")] {
        assert_eq!(route.uri(), uri);
        assert_eq!(route.way(), Way::Post, "{uri}");
        assert!(page::ORDER.contains(&route), "{uri} is never registered");
    }
}

#[test]
fn the_dictionary_has_its_path_and_is_registered_once_before_the_wildcard() {
    let dictionary = Route::ALL.iter().copied().find(|r| r.uri() == "/dictionary").expect("no path answers the dictionary");
    assert_eq!(dictionary.way(), Way::Get);
    assert_eq!(page::ORDER.iter().filter(|r| **r == dictionary).count(), 1, "the dictionary is not registered once");
    assert_eq!(page::refused(&page::ORDER), vec![]);
}

#[test]
fn the_pages_table_of_texts_is_served_as_json_and_a_name_that_leaves_the_page_is_not() {
    // 6.4: the words of a page of contract 3 lie in its kit as texts.json, and a browser reads it with fetch
    assert_eq!(page::mime("texts.json"), Some("application/json; charset=utf-8"));
    assert_eq!(page::mime("texts.json.gz"), Some("application/json; charset=utf-8"));
    assert_eq!(page::mime("../texts.json"), None);
    assert_eq!(page::mime("texts.jsonx"), None);
}

/// The order of 20.09 that took four images down: the wildcard first, everything else after it.
fn the_order_that_rolled_four_images_back() -> Vec<Route> {
    let mut order = vec![Route::Rest];
    order.extend(Route::ALL.iter().copied().filter(|r| *r != Route::Rest));
    order
}

/// A curve as a module hands one over: one line of a pin reading hundreds of millivolts.
fn curve_of(points: Vec<f32>) -> page::Curve {
    page::Curve {
        step_ms: 101,
        lines: vec![page::Line {
            words: "what the pin held".into(),
            unit: "mV".into(),
            points,
            span: Some((0.0, 3300.0)),
        }],
    }
}

#[test]
fn nothing_answers_the_reset_of_the_energy_by_a_path_of_its_own() {
    // the reset is a command of a row of the dictionary, pressed by its address like every other
    assert!(Route::ALL.iter().all(|r| r.uri() != "/reset-energy"), "a path of its own is left");
}

#[test]
fn the_busy_answer_says_two_pages_are_open() {
    assert_eq!(BUSY, "the board is busy: two pages are open on it already - close one and reload this one");
}
