//! Free text as a value of fixed length: a value a module publishes is copied with no heap, so its
//! text is as long as the longest the device has - a setting's value - and a longer one is cut where a character ends.

use page::dictionary::{Said, Text, LONGEST_TEXT};

#[test]
fn a_text_within_the_limit_passes_as_it_is() {
    for said in ["", "pool.ntp.org", "the gate refused 8 %", "котельная"] {
        assert_eq!(Text::from(said).as_str(), said);
    }
    let whole = "x".repeat(LONGEST_TEXT);
    assert_eq!(Text::from(whole.as_str()).as_str(), whole, "the limit itself is not cut");
}

#[test]
fn a_text_over_the_limit_is_cut_and_says_so_at_its_end() {
    let long = "a".repeat(LONGEST_TEXT + 1);
    let cut = Text::from(long.as_str());
    let kept = cut.as_str().strip_suffix('…').expect("a cut text ends with the mark of the cut");
    assert!(cut.as_str().len() <= LONGEST_TEXT, "{} bytes", cut.as_str().len());
    assert!(long.starts_with(kept), "what is kept is the beginning of what was said");
    assert!(!kept.is_empty());
}

#[test]
fn a_text_is_cut_where_a_character_ends_and_not_inside_one() {
    // two bytes a letter: the limit falls between the bytes of one of them
    let long = "é".repeat(LONGEST_TEXT);
    let cut = Text::from(long.as_str());
    let kept = cut.as_str().strip_suffix('…').expect("cut");
    assert!(kept.chars().all(|c| c == 'é'), "only whole letters are kept: {kept:?}");
    assert!(cut.as_str().len() <= LONGEST_TEXT);
}

#[test]
fn a_cut_text_goes_on_the_wire_as_any_text() {
    let long = "b".repeat(LONGEST_TEXT * 2);
    let text = Text::from(long.as_str());
    let wire = serde_json::to_value(Said::Text(text.as_str())).unwrap();
    let said = wire["text"].as_str().expect("the contract's {\"text\": …}");
    assert!(said.ends_with('…') && said.len() <= LONGEST_TEXT, "{said}");
}
