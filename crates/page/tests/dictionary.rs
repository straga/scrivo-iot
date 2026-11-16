//! The dictionary of the contract the document names: what each address is, for machines, and nothing a person reads.

use page::dictionary::{Count, self, Entry, Kind, Said, Value};

/// A table of the shapes the burner has: a code of a list, a reading with a part, a switch, a number a command sets.
static TABLE: [Entry; 6] = [
    Entry { words: &["out", "weak", "steady"], reasons: &["crossed"], ..Entry::row("flame", "flame", &[Kind::Word, Kind::Unknown]) },
    Entry { unit: "mV", curves: &["flame_fast"], ..Entry::row("flame", "pin", &[Kind::Number]) },
    Entry { unit: "mV", ..Entry::part("flame", "pin", "spread", &[Kind::Number]) },
    Entry { can: &["switch", "toggle"], ..Entry::row("relays", "pump", &[Kind::On]) },
    Entry {
        unit: "Pa",
        words: &["off", "full", "by_hand"],
        can: &["hold"],
        range: Some(dictionary::Range { min: 0, max: 300, step: 1, at: "asked" }),
        ..Entry::row("fan", "draught", &[Kind::Number, Kind::Word])
    },
    Entry { unit: "Pa", ..Entry::part("fan", "draught", "asked", &[Kind::Number]) },
];
static SYSTEM: [Entry; 1] = [Entry { unit: "consoles", ..Entry::row("system", "console", &[Kind::Number]) }];
static JOINED: [Entry; 7] = dictionary::joined(&SYSTEM, &TABLE);
/// An address as a module writes it: worked out when the image is built, from the table the dictionary is made of.
const PUMP: usize = dictionary::at(&JOINED, "relays", "pump");
const SPREAD: usize = dictionary::part_at(&JOINED, "flame", "pin", "spread");

static SETTINGS: [dictionary::Setting; 3] = [
    dictionary::Setting {
        name: "time",
        card: None,
        default: "auto",
        shape: dictionary::Shape::Choice(&[dictionary::Variant::plain("auto", false), dictionary::Variant::plain("internet", false), dictionary::Variant::plain("server", true)]),
        tuning: false,
        effect: None,
    },
    dictionary::Setting { name: "igniter_limit", card: Some("relays"), default: "360", shape: dictionary::Shape::Seconds { from: 0, to: 3600 }, tuning: false, effect: None },
    dictionary::Setting { name: "ntc_address", card: Some("temperatures"), default: "1", shape: dictionary::Shape::Whole { from: 1, to: 247, unit: "" }, tuning: false, effect: None },
];

fn meta() -> dictionary::Meta<'static> {
    dictionary::Meta { device: "burner", board: "STRAGA_BASE_S3", kit: "stragabases3-26.09.24.14", cards: &["flame", "relays", "fan"], settings: &SETTINGS, machine: None }
}

fn dictionary_of(entries: &[Entry]) -> serde_json::Value {
    serde_json::from_str(&dictionary::json(entries, &meta())).unwrap()
}

#[test]
fn the_dictionary_served_and_the_addresses_answered_are_made_of_one_table() {
    let json = dictionary_of(&JOINED);
    let things = json["things"].as_array().expect("things");
    assert_eq!(things.len(), JOINED.len());
    for (at, entry) in JOINED.iter().enumerate() {
        assert_eq!((things[at]["card"].as_str(), things[at]["name"].as_str()), (Some(entry.card), Some(entry.name)), "address {at}");
        if entry.part_of.is_none() {
            assert_eq!(dictionary::address(&JOINED, entry.card, entry.name), Some(at));
        }
    }
    assert_eq!(things[PUMP]["name"], "pump");
    assert_eq!(things[PUMP]["can"], serde_json::json!(["switch", "toggle"]));
    assert_eq!(things[SPREAD]["part_of"], "pin");
    assert_eq!(things[5]["range"], serde_json::json!({"min": 0, "max": 300, "step": 1, "at": 6}));
    assert_eq!(things[1]["words"], serde_json::json!(["out", "weak", "steady"]));
    assert_eq!(json["contract"], 10);
    assert_eq!(json["cards"], serde_json::json!(["system", "settings", "other", "flame", "relays", "fan"]));
}

#[test]
fn a_thing_the_description_has_not_got_has_no_address() {
    assert_eq!(dictionary::address(&JOINED, "relays", "auger"), None);
    assert_eq!(dictionary::address(&JOINED, "flame", "spread"), None, "a part is no row of its card");
    assert_eq!(dictionary::part(&JOINED, 2, "width"), None);
    assert_eq!(dictionary::part(&JOINED, 2, "spread"), Some(SPREAD));
}

#[test]
fn the_version_is_the_image_and_a_print_of_the_table() {
    let version = dictionary_of(&JOINED)["version"].as_str().unwrap().to_string();
    let (kit, print) = version.split_once('+').expect("kit+print");
    assert_eq!(kit, "stragabases3-26.09.24.14");
    assert_eq!(print.len(), 8);
    let mut other = TABLE;
    other[0].words = &["out", "weak", "steady", "blinking"];
    let other: [Entry; 7] = dictionary::joined(&SYSTEM, &other);
    assert_ne!(dictionary_of(&other)["version"], version.as_str(), "another code, the same version");
}

#[test]
fn a_sound_description_is_taken() {
    assert_eq!(dictionary::check(&JOINED), Ok(()));
}

#[test]
fn a_description_that_contradicts_itself_or_carries_words_is_refused_by_name() {
    let with = |change: &dyn Fn(&mut Vec<Entry>)| {
        let mut table = JOINED.to_vec();
        change(&mut table);
        dictionary::check(&table).expect_err("a broken description taken")
    };
    let refusals = [
        (with(&|t| t.push(Entry::row("relays", "pump", &[Kind::On]))), "relays/pump", "twice"),
        (with(&|t| t.push(Entry::part("relays", "fan", "speed", &[Kind::Number])) ), "relays/fan.speed", "no such row"),
        (with(&|t| t[3].unit = ""), "flame/pin.spread", "unit"),
        (with(&|t| t[1].name = "flame word"), "flame/flame word", "no code"),
        (with(&|t| t[1].words = &["out", "weak - lit, not steady"]), "flame/flame", "no code"),
        (with(&|t| t[1].reasons = &["Crossed"]), "flame/flame", "no code"),
        (with(&|t| t[5].range = Some(dictionary::Range { min: 0, max: 300, step: 1, at: "held" })), "fan/draught", "held"),
        (with(&|t| t[1].fixed = Some(1.0)), "flame/flame", "fixed"),
    ];
    for (refused, who, why) in refusals {
        assert!(refused.contains(who) && refused.contains(why), "expected {who} / {why}: {refused}");
    }
}

#[test]
fn a_value_goes_on_the_wire_short_its_kind_being_the_dictionarys() {
    let wire = |v: Value| serde_json::to_string(&v).unwrap();
    assert_eq!(wire(Value::On(true)), "true");
    assert_eq!(wire(Value::Number(61.5)), "61.5");
    assert_eq!(wire(Value::Seconds(52)), "52");
    assert_eq!(wire(Value::Word("kindling")), "\"kindling\"");
    assert_eq!(wire(Value::Words(&["auger", "pump"])), "[\"auger\",\"pump\"]");
    assert_eq!(wire(Value::AtLeast(300.0)), "{\"at_least\":300.0}");
    assert_eq!(wire(Value::Unknown("own_zero")), "{\"unknown\":\"own_zero\"}");
    assert_eq!(serde_json::to_string(&Said::Text("the numbers do not hold")).unwrap(), "{\"text\":\"the numbers do not hold\"}");
    assert_eq!(wire(Value::None), "null");
}

#[test]
fn the_settings_are_said_by_their_shape_and_their_codes_and_by_no_word() {
    let json = dictionary_of(&JOINED);
    assert_eq!(
        json["settings"],
        serde_json::json!([
            {"name": "time", "default": "auto", "kind": "choice", "choices": [{"value": "auto"}, {"value": "internet"}, {"value": "server", "takes": true}]},
            {"name": "igniter_limit", "card": "relays", "default": "360", "kind": "seconds", "from": 0, "to": 3600},
            {"name": "ntc_address", "card": "temperatures", "default": "1", "kind": "whole", "from": 1, "to": 247},
        ])
    );
}

#[test]
fn a_value_sent_only_when_a_reader_opens_says_so_to_the_reader() {
    static UPTIME: [Entry; 2] = [
        Entry { unit: "ms", once: true, ..Entry::row("system", "uptime", &[Kind::Number]) },
        Entry { unit: "bytes", ..Entry::row("system", "free", &[Kind::Number]) },
    ];
    let json = dictionary_of(&UPTIME);
    assert_eq!(json["things"][0]["once"], true);
    assert!(json["things"][1].get("once").is_none(), "a value sent whenever it changes carries no mark");
}

#[test]
fn a_card_counts_its_rows_and_their_parts_and_a_card_there_is_not_counts_nothing() {
    const FAN: usize = dictionary::addresses(&TABLE, "fan");
    assert_eq!(FAN, 2, "the draught and its part asked");
    assert_eq!(dictionary::addresses(&TABLE, "flame"), 3);
    assert_eq!(dictionary::addresses(&TABLE, "boiler"), 0);
}

#[test]
fn a_setting_folded_under_the_everyday_ones_says_so_in_the_dictionary_and_the_others_say_nothing() {
    static FOLDED: [dictionary::Setting; 2] = [
        dictionary::Setting { name: "everyday", card: Some("fan"), default: "1", shape: dictionary::Shape::Whole { from: 1, to: 9, unit: "" }, tuning: false, effect: None },
        dictionary::Setting { name: "folded", card: Some("fan"), default: "1", shape: dictionary::Shape::Whole { from: 1, to: 9, unit: "" }, tuning: true, effect: None },
    ];
    let mut on = meta();
    on.settings = &FOLDED;
    let json: serde_json::Value = serde_json::from_str(&dictionary::json(&JOINED, &on)).unwrap();
    let settings = json["settings"].as_array().expect("settings");
    assert_eq!(settings[0].get("level"), None, "an everyday setting was sent with a level");
    assert_eq!(settings[1]["level"], "tuning");
}

#[test]
fn a_device_with_no_folded_setting_serves_the_dictionary_it_served_before_the_level() {
    // no `level` anywhere in what it serves, so its version - the print of the whole - is not moved by the level existing
    let served = dictionary::json(&JOINED, &meta());
    assert!(!served.contains("level"), "{served}");
}

#[test]
fn a_choice_that_bears_on_something_says_what_and_a_plain_one_says_only_its_value() {
    static MODE: [dictionary::Variant; 2] = [
        dictionary::Variant::plain("level", false),
        dictionary::Variant {
            value: "by_a_sensor",
            takes: false,
            shows: &["hold_c"],
            needs: Some(dictionary::Needs { among: &["socket_1", "socket_2"], holds: "boiler" }),
        },
    ];
    static SETTINGS_OF_A_MODE: [dictionary::Setting; 1] =
        [dictionary::Setting { name: "mode", card: Some("fan"), default: "level", shape: dictionary::Shape::Choice(&MODE), tuning: false, effect: None }];
    let mut on = meta();
    on.settings = &SETTINGS_OF_A_MODE;
    let json: serde_json::Value = serde_json::from_str(&dictionary::json(&JOINED, &on)).unwrap();
    let choices = json["settings"][0]["choices"].as_array().unwrap();
    assert_eq!(choices[0], serde_json::json!({"value": "level"}), "a choice that bears on nothing is what it was");
    assert_eq!(choices[1]["shows"], serde_json::json!(["hold_c"]));
    assert_eq!(choices[1]["needs"], serde_json::json!({"among": ["socket_1", "socket_2"], "holds": "boiler"}));
}

#[test]
fn a_device_whose_choices_bear_on_nothing_serves_the_dictionary_it_served_before() {
    // its version - the print of the whole - is not moved by the fields existing
    let served = dictionary::json(&JOINED, &meta());
    assert!(!served.contains("shows") && !served.contains("needs"), "{served}");
}

#[test]
fn a_board_with_an_automaton_says_which_of_its_rows_are_which_and_one_without_says_nothing() {
    static MACHINE: dictionary::Machine = dictionary::Machine {
        card: "fan",
        fixed: "flame",
        stage: "holding",
        says: "holding",
        cause: "holding",
        press: &["holding"],
        aside: &[],
        sections: &[dictionary::Section { name: "first", by: "", items: &[&["one"], &["two", "three"]] }],
        now: &[],
    };
    let mut on = meta();
    on.machine = Some(&MACHINE);
    let json: serde_json::Value = serde_json::from_str(&dictionary::json(&JOINED, &on)).unwrap();
    assert_eq!(json["machine"]["card"], "fan");
    assert_eq!(json["machine"]["press"], serde_json::json!(["holding"]));
    assert_eq!(json["machine"]["sections"], serde_json::json!([{"name": "first", "items": [["one"], ["two", "three"]]}]));
    let served = dictionary::json(&JOINED, &meta());
    assert!(!served.contains("machine"), "a board with no automaton serves the dictionary it served before: {served}");
}

#[test]
fn a_number_says_how_many_digits_it_keeps_a_count_says_which_way_it_goes_and_the_stream_says_its_pace() {
    static ROWS: [Entry; 4] = [
        Entry { unit: "Pa", decimals: Some(1), ..Entry::row("c", "pressure", &[Kind::Number]) },
        Entry { counts: Some(Count::Up), ..Entry::row("c", "for", &[Kind::Seconds]) },
        Entry { counts: Some(Count::Down), clock: true, ..Entry::row("c", "left", &[Kind::Seconds]) },
        Entry { unit: "mV", ..Entry::row("c", "plain", &[Kind::Number]) },
    ];
    let json = dictionary_of(&ROWS);
    let things = json["things"].as_array().unwrap();
    assert_eq!(things[0]["decimals"], 1);
    assert_eq!(things[1]["counts"], "up");
    assert_eq!(things[2]["counts"], "down");
    for (at, key) in [(0, "counts"), (1, "decimals"), (3, "decimals"), (3, "counts")] {
        assert!(things[at].get(key).is_none(), "thing {at} says '{key}' of what it does not have");
    }
    assert_eq!(json["stream"], serde_json::json!({"every": 5, "live_s": 600, "most": 60}));
    assert_eq!(dictionary::check(&ROWS), Ok(()));
}

#[test]
fn a_description_that_gives_digits_to_what_is_no_number_or_a_count_to_what_is_no_seconds_is_refused_by_name() {
    let refused = |entry: Entry| dictionary::check(&[entry]).expect_err("a broken description taken");
    let (not_a_number, not_seconds, once, too_many) = (
        refused(Entry { decimals: Some(1), ..Entry::row("c", "word", &[Kind::Word]) }),
        refused(Entry { unit: "u", counts: Some(Count::Up), ..Entry::row("c", "number", &[Kind::Number]) }),
        refused(Entry { counts: Some(Count::Up), once: true, ..Entry::row("c", "once", &[Kind::Seconds]) }),
        refused(Entry { unit: "u", decimals: Some(7), ..Entry::row("c", "digits", &[Kind::Number]) }),
    );
    for (why, who) in [(not_a_number, "c/word"), (not_seconds, "c/number"), (once, "c/once"), (too_many, "c/digits")] {
        assert!(why.contains(who), "expected {who}: {why}");
    }
}

/// What a client is told about a command that cannot be taken back: the row says so, and a client of the contract asks before it sends the command.
static IRREVERSIBLE: [Entry; 3] = [
    Entry { can: &["press"], irreversible: true, ..Entry::row("energy", "reset", &[Kind::Button]) },
    Entry { can: &["press"], ..Entry::row("energy", "mark", &[Kind::Button]) },
    Entry { can: &["switch"], ..Entry::row("energy", "lamp", &[Kind::On]) },
];

#[test]
fn a_command_that_cannot_be_taken_back_says_so_and_the_commands_that_can_say_nothing() {
    let meta = dictionary::Meta { device: "probe", board: "PROBE", kit: "probe-1", cards: &["energy"], settings: &[], machine: None };
    let json: serde_json::Value = serde_json::from_str(&dictionary::json(&IRREVERSIBLE, &meta)).unwrap();
    let things = json["things"].as_array().unwrap();
    assert_eq!(things[0]["irreversible"], true);
    assert!(things[1].get("irreversible").is_none(), "a press that can be undone says nothing: {}", things[1]);
    assert!(things[2].get("irreversible").is_none(), "{}", things[2]);
}

#[test]
fn a_row_that_takes_no_command_cannot_be_called_irreversible() {
    let table = [Entry { irreversible: true, ..Entry::row("energy", "total", &[Kind::Number]) }];
    let said = dictionary::check(&table).unwrap_err();
    assert!(said.contains("energy/total") && said.contains("irreversible"), "{said}");
    assert_eq!(dictionary::check(&IRREVERSIBLE), Ok(()));
}
