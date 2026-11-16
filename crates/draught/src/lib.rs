//! What the burner's pressure sensor says in one reading of the pin, in pascals, and nothing about wires. The
//! ADC that fetches the millivolts is the device's half (devices/burner/src).
//!
//! The sensor is a Huba 401.93000: 0…3 mbar, which is 0…300 Pa, out on 0.5…4.5 V. Read off its own label on
//! 12.09; no datasheet was found, and the tolerance of its scale and of its zero is therefore not named.
//!
//! **WE BLOW, WE DO NOT DRAW** - the owner's word, 20.09. The fan pushes air into the burner and there is no
//! extractor in this boiler at all, so the pressure is positive. Whether this particular sensor is differential
//! or absolute is not established, and the direction is settled by switching the fan on and watching which way
//! the number goes - not by reasoning from a label.

/// Where the sensor puts its own zero, and where its top is. THE ZERO IS OFFSET ON PURPOSE: a sensor saying "no
/// pressure" says half a volt, a sensor that is not there says nothing at all, and the two are different
/// readings. Reading the second as the first is how a regulator drives a fan at full against a draught it cannot
/// see.
pub const ZERO_MV: u32 = 500;
pub const FULL_MV: u32 = 4_500;
/// What the top of that span is worth.
pub const FULL_PA: u32 = 300;

/// The largest either arm of the divider may be.
///
/// **A MEGAOHM, AND IT IS THE SAME NUMBER THE CARD ALLOWS** (`burner_settings::DIVIDER_TOP`): over a megaohm the ADC's
/// own input impedance starts to be part of the divider, and the arithmetic in this crate stops describing the
/// board. Two caps that disagreed would mean a pair a person can type and the reading cannot use.
///
/// It is also what keeps the arithmetic below honest by construction. With both arms capped, the pin's tenths
/// times the span is at most about 7e8 - nowhere near a u64 - so no product here can wrap, and there is no
/// narrowing anywhere for a bogus pressure to come out of.
pub const MOST_OHM: u32 = 1_000_000;

/// Below this the sensor is not answering at all. Half of its own zero: a broken wire gives 0 mV and a live
/// sensor never goes under its offset by more than drift, so the line sits between the two with room on both
/// sides.
pub const MISSING_BELOW_MV: u32 = ZERO_MV / 2;

/// What one reading amounts to. Four outcomes and no fifth, and none of the last three is ever a number: a
/// pressure of zero is what a still burner reads, and a person - or a regulator - would act on it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Reading {
    /// The pressure, in pascals, as a FRACTION - like the readings of `max6675` and `modbus`, which are f32 for
    /// the same reason. One millivolt on the pin is 0.15 Pa through a divider of a half, so a reading rounded to
    /// whole pascals steps by one, and at a setpoint of twenty pascals that step is something a regulator would
    /// chase (`crates/regulator`).
    Is(f32),
    /// The sensor answers, but under its own zero: drift, or suction where we expect blowing.
    Under,
    /// Above the top of its scale. Named rather than clamped: 300 Pa and "more than this sensor can say" are
    /// different facts, and a regulator that cannot tell them apart will chase the ceiling.
    Over,
    /// Nothing on the pin. Not a pressure.
    Missing,
    /// The divider it was asked to read through is not a divider. NOT THE SAME FAULT AS A SENSOR THAT FELL OFF,
    /// and the two used to look alike: a bottom arm of nothing gave zero millivolts, and zero reads as Missing.
    /// One of them sends a person to the soldering iron and the other to a line of configuration.
    Unconfigured,
}

/// What the SENSOR put out, from what the PIN saw, given the two resistors of the divider.
///
/// The coefficient is the resistors and not a number written down once: if the measurement
/// does not come out at half, the resistors are measured and the real value is what gets written. So the real
/// value is what this takes.
pub fn sensor_mv(pin_mv: u32, top_ohm: u32, bottom_ohm: u32) -> Option<u32> {
    // NOTHING, AND NOT THE NUMBER ZERO. A divider with no bottom arm is not a divider that reads zero volts -
    // it is a setting nobody filled in, and answering it with a number is how that got read as a sensor lying
    // on the floor. The sum is taken in u64 because two resistances that do not fit a u32 would wrap into a
    // small one and come back as a perfectly ordinary pressure.
    let span = top_ohm as u64 + bottom_ohm as u64;
    if !is_a_divider(top_ohm, bottom_ohm) {
        return None;
    }
    // AND THE RESULT IS CHECKED TOO, not only the sum. A pair of arms whose sum fits a u32 can still send the
    // millivolts of the sensor past one: 4 292 115 452 over 35 at 1506 mV on the pin works out to
    // 184 683 597 812, of which a u32 keeps 4084 - and 4084 mV is 268 Pa, a draught this boiler could have. A
    // check that closes one door of a class and leaves another open is worse than no check, because it is
    // believed.
    let said = pin_mv as u64 * span / bottom_ohm as u64;
    (said <= u32::MAX as u64).then_some(said as u32)
}

/// One reading of the pin, through the divider it is behind: the two halves in one call, so a caller cannot get
/// the order of them wrong.
pub fn read(pin_mv: u32, top_ohm: u32, bottom_ohm: u32) -> Reading {
    read_tenths(pin_mv * 10, top_ohm, bottom_ohm)
}

/// The same, from TENTHS of a millivolt on the pin.
///
/// TWO ENTRY POINTS BECAUSE TWO RESOLUTIONS REALLY EXIST, not because two answers do. A window of ADC samples
/// is averaged in tenths (`analog::Window::point`), and throwing that away before the arithmetic is what put a
/// quiet pin on a page as square teeth. The unit is in the NAME: a caller that mixes them up is out by ten, and
/// a reading out by ten of a sensor that reads 0…300 Pa is still an ordinary pressure.
pub fn read_tenths(pin_mv_tenths: u32, top_ohm: u32, bottom_ohm: u32) -> Reading {
    match sensor_tenths(pin_mv_tenths, top_ohm, bottom_ohm) {
        Some(tenths) => pascals_of_tenths(tenths),
        None => Reading::Unconfigured,
    }
}

/// What the sensor's TENTHS of a millivolt mean in pascals, from a number that is allowed to be enormous.
///
/// **THE GUARD AGAINST A BOGUS PRESSURE IS THE WINDOW, NOT A CHECK ON A NARROWING.** A pressure comes out only
/// for tenths INSIDE the sensor's own span; everything under it is Missing or Under and everything over it is
/// Over, whatever the arithmetic above produced. So the old fourth door - a result too large for a u32 coming
/// back as an ordinary draught - is not closed by a check here, it does not exist: there is nothing to narrow.
pub fn pascals_of_tenths(sensor_tenths: u64) -> Reading {
    if sensor_tenths < (MISSING_BELOW_MV * 10) as u64 {
        return Reading::Missing;
    }
    if sensor_tenths < (ZERO_MV * 10) as u64 {
        return Reading::Under;
    }
    if sensor_tenths > (FULL_MV * 10) as u64 {
        return Reading::Over;
    }
    let over_zero = (sensor_tenths - (ZERO_MV * 10) as u64) as f32 / 10.0;
    Reading::Is(over_zero * FULL_PA as f32 / (FULL_MV - ZERO_MV) as f32)
}

/// What the sensor's millivolts mean in pascals.
pub fn pascals(sensor_mv: u32) -> Reading {
    pascals_of_tenths(sensor_mv as u64 * 10)
}

/// What the SENSOR put out in tenths of a millivolt, from the tenths the PIN saw, or None when the pair of arms
/// is not a divider at all.
///
/// The product is taken in u64 and NOT narrowed: a pair whose sum fits a u32 and a pin that fits a u32 make at
/// most 1.8e19, which a u64 holds. What the number then means is `pascals_of_tenths`, and a number past the
/// sensor's span is a reading over its scale rather than a fault of the setting.
fn sensor_tenths(pin_tenths: u32, top_ohm: u32, bottom_ohm: u32) -> Option<u64> {
    if !is_a_divider(top_ohm, bottom_ohm) {
        return None;
    }
    let span = top_ohm as u64 + bottom_ohm as u64;
    Some(pin_tenths as u64 * span / bottom_ohm as u64)
}

/// Whether these two arms are a divider this crate can read through at all.
///
/// Nothing at the bottom is not a divider that reads zero volts - it is a setting nobody filled in. An arm past
/// `MOST_OHM` is not one either, and the fault is named by ITS OWN CAUSE: answering such a pair with "over the
/// sensor's scale" would put "turn the fan down" on the card, where what is wanted is a line of configuration.
pub fn is_a_divider(top_ohm: u32, bottom_ohm: u32) -> bool {
    bottom_ohm > 0 && top_ohm <= MOST_OHM && bottom_ohm <= MOST_OHM
}

/// What a reading says to a person, on the card and in the shell.
///
/// FOUR OF THE FIVE ARE NOT A NUMBER, and none of them may come out as 0 or as a dash: a card that does that
/// tells a person the burner is fine and reads nothing. So each says what it is AND what to do about it - the
/// device says its present state and what to do with it, never its history (the domain's charter).
///
/// SHORT WORDS. This is a row in a card on a phone, and the first version of these - 130 characters of good
/// advice - wrapped to five lines where a person expects a reading (runs/3.3/draught_card_375.png). What to do
/// fits in a few words when the fields it points at are right there under the row.
pub fn words(said: Reading) -> String {
    match said {
        // WHOLE PASCALS ON THE CARD, and the fraction stays inside for the regulator: the noise here is about
        // three pascals at small travel, so a tenth on a row is a digit of breathing.
        Reading::Is(pa) => format!("{pa:.0} Pa"),
        Reading::Under => format!("under {ZERO_MV} mV, its own zero - check the tube's port"),
        Reading::Over => format!("over {FULL_PA} Pa - turn the fan down"),
        Reading::Missing => format!("no sensor - under {MISSING_BELOW_MV} mV; check its 12 V"),
        Reading::Unconfigured => "not a divider - fill both resistances below".to_string(),
    }
}

/// The row a person reads while the regulator holds: what came out, of what was asked, at what angle.
///
/// **A FAULT IS NEVER PUT WHERE A NUMBER IS EXPECTED.** `words` answers a sentence for everything that is not
/// a pressure - "under 500 mV, its own zero - check the tube's port" - and dropped into the slot the pascals
/// live in, it reads as a pressure: on 21.09 the row said "under 500 mV, its own zero - check the tube's port
/// of 30 Pa asked, at 33 % of air", which lies to a person at a boiler twice over, about the fault and about
/// the value. So a fault opens the row and stands whole; what the fan is doing meanwhile follows it, fenced
/// off by a semicolon. What to fix comes before what the fan is at.
pub fn holding_words(said: Reading, asked_pa: f32, angle: f32) -> String {
    match said {
        Reading::Is(pa) => format!("{pa:.0} Pa of {asked_pa:.0} Pa asked, at {angle:.0} % of air"),
        other => format!("{}; the fan holds {angle:.0} % of air, {asked_pa:.0} Pa asked", words(other)),
    }
}

/// What the slider's own row says: the pressure being held for now, and - when it is not the kept one - the
/// pressure the board would come up at, with what to press to make them one.
///
/// **THE TWO NUMBERS ARE THE PRICE OF COMMANDING AND SAVING BEING DIFFERENT ACTS.** While the slider both
/// asked and saved there was one number, and the row needed to say nothing about keeping. Now a person can
/// leave the card holding 60 Pa while it comes up at 51, and a card that shows only one of them is a card
/// that surprises them at the next restart. So the row names the state as it is - what is held, what would
/// come back - and what to press, since a line that reports a difference without saying how to close it
/// leaves a person looking for the button.
///
/// Said only when they DIFFER. Equal, there is one number and one clause, and a second one repeating it would
/// teach the reader to stop reading the line.
pub fn asked_words(asked_pa: f32, kept_pa: u32) -> String {
    let asked = asked_pa.round() as i64;
    if asked == kept_pa as i64 {
        return format!("{asked} Pa asked");
    }
    format!("{asked} Pa asked; comes up at {kept_pa} Pa - Save on this card keeps {asked}")
}

/// The same row before the sensor has answered at all - a second after a start. Not having read yet is a state
/// of the reading like any other, so it is said the same way and never flows into the number's grammar.
pub fn holding_words_unread(asked_pa: f32, angle: f32) -> String {
    format!("not read yet; the fan holds {angle:.0} % of air, {asked_pa:.0} Pa asked")
}

/// What the PIN sees when the SENSOR puts out this much, through the divider of these two resistors - the other
/// way round from `sensor_mv`.
///
/// This is what stands beside the two fields of the divider once a person has filled them in: with no pressure
/// the sensor puts out `ZERO_MV`, so the card can say which millivolts the pin should sit at. Comparing that
/// with the millivolts the pin actually shows is the whole of the measurement, and it needs no arithmetic done
/// in anyone's head.
pub fn pin_mv(sensor_mv: u32, top_ohm: u32, bottom_ohm: u32) -> Option<u32> {
    if !is_a_divider(top_ohm, bottom_ohm) {
        return None;
    }
    let span = top_ohm as u64 + bottom_ohm as u64;
    // This one cannot overflow the way `sensor_mv` could, and the reason is worth writing down rather than
    // rediscovering: the pin only ever sees a SHARE of the sensor, `bottom` is part of `span`, so the result is
    // never larger than what went in. The other direction multiplies by a share greater than one, which is why
    // it needs a check and this does not.
    Some((sensor_mv as u64 * bottom_ohm as u64 / span) as u32)
}
