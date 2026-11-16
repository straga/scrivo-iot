//! What a sensor keeps between rounds: the last reading, the curves, whether the card has news, the step.

use module::sensor::{Kept, Sensor};

/// A sensor of the tests: its reading is a pressure in tenths, the card shows whole units, and the curve keeps
/// the points of readings that are numbers.
struct Gauge;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Said {
    Is(u16),
    Silent,
}

impl Sensor for Gauge {
    type Reading = Said;
    type Curves = Vec<u16>;
    type Shown = Option<Option<u16>>;

    fn read(&mut self) -> Option<Said> {
        None
    }

    fn record(curves: &mut Vec<u16>, reading: &Said) {
        if let Said::Is(tenths) = reading {
            curves.push(*tenths);
        }
    }

    fn shown(reading: Option<&Said>) -> Self::Shown {
        reading.map(|r| match r {
            Said::Is(tenths) => Some(tenths / 10),
            Said::Silent => None,
        })
    }
}

#[test]
fn a_reading_is_kept_as_the_last_and_recorded_in_the_curves() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    kept.take(Some(Said::Is(123)));
    kept.take(Some(Said::Is(456)));
    assert_eq!(kept.last(), Some(Said::Is(456)));
    assert_eq!(kept.curves(|c| c.clone()), vec![123, 456]);
}

#[test]
fn nothing_read_is_kept_as_nothing_and_draws_nothing() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    assert_eq!(kept.last(), None, "a sensor never read has a last reading");
    kept.take(Some(Said::Is(123)));
    kept.take(None);
    assert_eq!(kept.last(), None, "a round that read nothing left the reading before it standing");
    kept.take(Some(Said::Silent));
    assert_eq!(kept.curves(|c| c.clone()), vec![123], "a reading that is not a number went into the curve");
}

#[test]
fn the_card_has_news_when_what_it_shows_changes() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    assert!(kept.take(Some(Said::Is(120))), "the first reading is news");
    assert!(kept.take(Some(Said::Is(130))), "12 to 13 is news");
    assert!(kept.take(Some(Said::Silent)), "a number to silence is news");
    assert!(kept.take(None), "silence to nothing read is news");
}

#[test]
fn a_reading_that_shows_the_same_is_no_news() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    kept.take(Some(Said::Is(120)));
    assert!(!kept.take(Some(Said::Is(129))), "12.0 and 12.9 both show 12");
    assert!(!kept.take(Some(Said::Is(120))));
    kept.take(None);
    assert!(!kept.take(None), "nothing read twice is news");
}

#[test]
fn the_step_is_the_asked_one_until_one_is_measured() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    assert_eq!(kept.step_ms(), 100);
    kept.measured(109);
    assert_eq!(kept.step_ms(), 109);
}

#[test]
fn a_measured_step_of_zero_is_not_taken() {
    let kept: Kept<Gauge> = Kept::new(Vec::new(), 100);
    kept.measured(0);
    assert_eq!(kept.step_ms(), 100, "a curve drawn on a step of zero has all its points at one moment");
}

/// A sensor whose card shows its reading through a setting - as the flame shows millivolts through its thresholds.
struct Scaled;

static SCALE: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(10);

impl Sensor for Scaled {
    type Reading = u16;
    type Curves = ();
    type Shown = u16;

    fn read(&mut self) -> Option<u16> {
        None
    }

    fn record(_curves: &mut (), _reading: &u16) {}

    fn shown(reading: Option<&u16>) -> u16 {
        reading.map_or(u16::MAX, |r| r / SCALE.load(std::sync::atomic::Ordering::Relaxed))
    }
}

#[test]
fn a_setting_that_changes_what_the_card_shows_is_news_with_the_same_reading() {
    let kept: Kept<Scaled> = Kept::new((), 100);
    SCALE.store(10, std::sync::atomic::Ordering::Relaxed);
    kept.take(Some(500));
    assert!(!kept.take(Some(500)), "the same reading through the same setting is news");
    SCALE.store(100, std::sync::atomic::Ordering::Relaxed);
    assert!(kept.take(Some(500)), "the card shows 5 where it showed 50, and nobody was told");
}
