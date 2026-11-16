//! What angle holds the draught at its setpoint. Pascals in, a percent of air out, and nothing about wires: the
//! sensor is `crates/draught`, the angle is `crates/dimmer`, and the pin, the ADC and the triac are the device's
//! half (devices/burner/src).
//!
//! **THE SETPOINT IS IN PASCALS AND NEVER IN PERCENT.** A percent is the position of a dimmer's knob and carries
//! no physical meaning: the owner measured 30 % giving 89 Pa and, within the hour, 47 Pa on the same cold burner.
//! The draught of a chimney moves with the weather and the motor behaves
//! differently cold and warm, so a table of "percent to pascals" does not exist and there is no point building
//! one. Only pascals repeat.
//!
//! **PI AND NOT PID.** Proportional and integral action forgive noise; derivative action amplifies it into the
//! output, and our noise is the breathing of the air itself - about 5 % of the value, and up to HALF the signal
//! at small travel, where 12 Pa wandered between 9 and 15. A derivative would be voting on the breathing. With
//! a tract that takes ten to fifteen seconds, the integral arrives in time by itself.
//!
//! **THE ERROR IS DIVIDED BY THE STEEPNESS OF THE FAN'S CURVE BEFORE ANYTHING ACTS ON IT.** The fan is not a
//! straight line: measured on the board, 15 % of angle settles at 26 Pa, 25 % at 75, 35 % at 141 and 50 % at
//! 245, so a percent of angle is worth 4.9 Pa low down the travel and 6.9 Pa high up, and the POWER of that
//! curve falls from 2.07 to 1.53 along it. Gains picked at one end are either sluggish or twitchy at the
//! other, and no tuning fixes that - it is the fan, not the setting. Divided by `n·pa/angle` with `n` read
//! off the band the angle is in ([`curve_power`]), the error stops being pascals and becomes percents of
//! ANGLE, and from there the fan looks the same everywhere.
//!
//! **INCREMENTAL FORM: the error moves the angle, it does not set it.** Two reasons, and both are ours. The
//! angle it starts from is the one the fan is ALREADY held at, so switching the regulator on does not jerk the
//! fan - on a live fire that would catch or smother the flame. And the response is not symmetric, so there are
//! two sets of gains; in this form the change between them is one small increment rather than a step in the
//! output (same research, section 3).

pub mod knobs;

use draught::Reading;

/// What the fan is being asked to do. Three, and confusing them is the thing this enum exists to stop.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Asked {
    /// The purge before lighting and after stopping: full air, no feedback wanted and none used. The sensor
    /// ends at 300 Pa and the purge runs at the maximum, so "over the scale" here is the ordinary reading of a
    /// working board and NOT a fault.
    Purge,
    /// Hold this many pascals. The regulator's own work.
    Hold(f32),
    /// Off - and that means zero, not a very late firing: fired late the triac still passes a sliver of every
    /// half cycle and the motor hums at it (crates/dimmer).
    Stop,
    /// **NOT ONE OF THE BURNER'S THREE MODES: the regulator is not taking part at all.** A person is turning the
    /// angle by hand, to probe the fan, and the regulator must leave it alone - which `Stop` does not do: `Stop`
    /// commands zero, and commanding zero once a second took the hand-set angle away before the fan had spun up
    /// (measured on the board 21.09, and the row read "gate off" throughout while a person was asking for 33 %).
    ByHand,
}

/// One set of gains, in THOUSANDTHS, because a person types whole numbers into a card.
///
/// **THEY ACT ON A MISS EXPRESSED IN PERCENTS OF ANGLE, NOT IN PASCALS.** [`Holding::slope`] divides the error
/// by how steeply the draught answers the angle where the fan stands now, so `kp` is thousandths of a percent
/// of angle per percent of miss - a plain ratio - and `ki` the same per second. What they SHOULD be is read
/// off the convergence curve on the stand, not guessed here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Gains {
    pub kp_milli: u32,
    pub ki_milli: u32,
}

/// Everything a person can turn, in one place.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuning {
    /// Pushing the pressure UP, and letting it DOWN: two sets, because the TRACT is not symmetric - a step of
    /// the angle reaches 63 % of its change in 4 s going up and in 3 s coming down. What these two are NOT
    /// about any more is the two ends of the travel: that difference is the shape of the fan, and
    /// [`Holding::slope`] divides it out before the gains ever see the error.
    pub up: Gains,
    pub down: Gains,
    /// The lowest angle the regulator may hold. A TURNING fan holds down to `dimmer::HOLDS_AT`, labouring; a
    /// standing one needs `dimmer::STARTS_AT` to break away, and that is the kick's business, not this floor's.
    pub min: f32,
    /// The highest. About 80 % of travel is where the measurement ends - the pin reads 2317 mV of a window that
    /// closes at 2450 and the sensor is already past its 300 Pa - so above it the regulator would be chasing a
    /// number that cannot move.
    pub max: f32,
    /// The most one step may move the angle, in percent. Without it a big error throws the angle across its
    /// whole range in one second, and the fan bangs.
    pub step: f32,
    /// How near the setpoint, in pascals, the gains act at `band_share` of themselves.
    ///
    /// **THE BREATHING OF THE AIR IS NOT A MISS.** A reading moves 1.7-1.9 Pa from one second to the next under fire
    /// and cold alike (26.09), and gains quick enough to bring the draught back from a jump within fifteen seconds
    /// answer that breathing with the gate as well. Inside the band they act at a share of themselves, outside it in
    /// full - ESPHome's pid does the same with its deadband and `kp_multiplier`/`ki_multiplier`.
    /// In this incremental form the change of gains at the edge
    /// moves only the next step, so crossing the edge does not jerk the angle.
    pub band_pa: f32,
    /// The share of their strength the gains act at inside the band, 0 to 1.
    pub band_share: f32,
}

/// Why the regulator is holding nothing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Blind {
    /// The sensor answers under its own zero: drift, or suction where we blow.
    Under,
    /// Nothing on the pin at all.
    Missing,
    /// The divider it is read through is not a divider - a setting nobody filled in.
    Unconfigured,
    /// The fan was kicked and made no draught: it is not turning, and the board must not pretend it is.
    NeverStarted,
}

/// What one step decided.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Held {
    /// Hold the fan at this percent of air.
    At(f32),
    /// Touch nothing: a person is holding the angle by hand. The one answer that is not a command.
    Alone,
    /// Nothing may be held, so the fan stops. **NOT "keep the last angle"**: on a live fire, fuel going into a
    /// pot the board believes is being blown is worse than a fan that stopped and said so.
    Blind(Blind),
}

/// What the fan is given to break away from rest, and for how long.
///
/// THE RANGE IS THE OWNER'S, MEASURED ON HIS OWN FAN (`dimmer::STARTS_AT`, `dimmer::HOLDS_AT`, 20.09): it will
/// not start at 7 %, it starts at 25, it runs evenly at 30, and once turning it holds down to 8. His word on the
/// kick is 30-35 % for a second; the middle of that is taken here. A smooth ramp from below does NOT work - an
/// induction motor needs torque at once, or it hums, heats and does not move.
pub const KICK_PERCENT: f32 = 33.0;

/// How long one kick blows before it is judged.
///
/// **FOUR SECONDS, AND THE NUMBER IS MEASURED ON THIS BOILER.** The owner's word was "give it a second and look
/// at the pressure", and the board says a second is not enough: held at 33 % from rest the draught read nothing
/// at 2 s, 27 Pa at 5 s, 90 Pa at 10 s and settled at 115 Pa by 20 s
/// (measured on the bench board with the gate held by hand at 33 %, kit stragabases3-26.09.21.29, 21.09.2026). So it
/// crosses the floor that proves turning somewhere between the second and the fifth second, and a window of one
/// condemns a fan that is in fact spinning up. Four is that crossing with a margin.
pub const KICK_SECONDS: f32 = 4.0;

/// How many kicks before the board says the fan is not starting. Three, so that one unlucky window - a reading
/// taken while the pressure was still building - does not condemn a fan that is in fact turning.
pub const KICKS: u32 = 3;

/// The pressure that PROVES the fan turns.
///
/// Five pascals, and the basis is the stage's own measurement: at 12 Pa the curve wandered between 9 and 15, so
/// the noise band at small travel is about ±3 Pa. Below that band a number says nothing about turning. A
/// standing fan does not reach it at all - it reads `Under`, or zero.
pub const TURNS_ABOVE_PA: f32 = 5.0;

/// How far above a stall the regulator puts its floor.
///
/// One percent. The angle it fell out at is known bad; the question is how much more is enough, and the
/// smallest step that can be told apart is the honest answer - a bigger margin is a guess, and this one is
/// corrected by the next stall anyway.
pub const STEP_ABOVE_A_STALL: f32 = 1.0;

/// How long the fan has to hold near its found floor before the floor is allowed a step down.
///
/// Thirty seconds, and it is a pace rather than a measurement of anything: what is being waited for is the
/// motor warming, which has no signal the board can read. Slow enough that it is not tried every other second,
/// short enough that a fan warming through a burn keeps getting its air back.
pub const FLOOR_SLIDES_AFTER_S: f32 = 30.0;

/// How long the tract takes to answer a change of the angle, as a first-order time constant.
///
/// **FOUR SECONDS, AND IT IS MEASURED.** `tools/tract_lag_probe.py` on the bench board, 21.09, with the gate
/// held by hand so that nothing moved the angle underneath: a step from 20 % to 35 % reached 63 % of its change
/// in 4 s and 95 % in 10 s; coming back down, 3 s and 9 s. The header of this module says "ten to fifteen
/// seconds" - that number was never measured, and the descent must not be built on it.
pub const TRACT_ANSWERS_S: f32 = 4.0;

/// The power of the fan's curve, `n` of `pa = k·angle^n`, WHERE THE FAN IS STANDING NOW.
///
/// **THERE IS NO ONE POWER FOR THIS FAN, AND THAT IS MEASURED RATHER THAN ASSUMED.** Three rungs on the board
/// with the gate held by hand (`tools/tract_lag_probe.py`, 21.09, the numbers in the arms below) came out at
/// 2.074, 1.916 and 1.529. The stage allowed a spread of 0.2 and got 0.545, and the noise bands of the outer
/// two do not even overlap - the fan saturates as the angle grows, and a single power read the top of the
/// travel as far livelier than it is: 8.7 Pa a percent predicted against 6.9 measured.
///
/// So the power is read off the band the angle is in. The live half stays live: `pa/angle` is still taken
/// from what the board reads this second, so the steepness keeps following the motor warming and the chimney
/// pulling differently. What the bands fix is the SHAPE, which a snapshot of one day cannot carry.
///
/// **THE BANDS ARE MEASURED UP TO 50 % OF TRAVEL AND NO FURTHER.** Above that the last band carries on with
/// the last power there is evidence for, and the power up there is not measured at all; the arm says what
/// follows if it is in fact lower. The regulator is shown to work for setpoints up to about 60 Pa, which it
/// holds at 22 % of travel.
///
/// **EVERY EDGE BETWEEN BANDS IS A STEP, and the step is exactly the ratio of the two powers** - the pressure
/// and the angle are the same on both sides of it. At 25 % that is 1.078, inside the tenth part the stage
/// asks for. At 35 % it is 1.255 and outside it: the fan's shape really does break there, and no numbers
/// within these three bands close it. The setpoints this regulator holds sit at 16 % and 22 % of travel, so
/// the working edge is the one at 25 %; smoothing the other is its own piece of work.
pub fn curve_power(angle: f32) -> f32 {
    match angle {
        // 15 % held 26 Pa and 25 % held 75
        a if a < 25.0 => 2.07,
        // 25 % held 74 Pa and 35 % held 141
        a if a < 35.0 => 1.92,
        // 35 % held 142 Pa and 50 % held 245.
        //
        // **NOTHING WAS MEASURED ABOVE 50 %, AND THE SETTING ALLOWS THE ANGLE UP TO 80.** This band carries
        // on into that stretch with the last power there is evidence for. Up there it is an extrapolation,
        // and what the real power is nobody has measured: the only hint is a draft rung 45 -> 50 % that came
        // out at 0.68, taken with 40 s a level, which is too short to call settled.
        //
        // What IS certain is the direction of the mistake if the power up there is in fact lower. The step of
        // the angle is the error divided by `n·pa/angle`, so it is inversely proportional to `n`: too high an
        // `n` gives too small a step, and the regulator moves the angle LESS than the fan needs. Too gentle,
        // never twitchier. The other way round - a real power ABOVE 1.53 - would make it fierce, and nothing
        // in the three measured rungs points that way: the power fell along the whole of them, 2.07, 1.92,
        // 1.53.
        //
        // Measuring it wants a rung above 50 %, and the sensor ends at 300 Pa - which the fan reaches at
        // about 57 % of travel, so the rung has to fit in the little room between them.
        //
        // A reading that is not a number lands here too, on the gentlest power of the three.
        _ => 1.53,
    }
}

/// The least steepness the regulator will divide by, in pascals per percent of angle.
///
/// **2.4, AND IT IS THE FAN'S OWN CURVE AT THE BOTTOM OF THE TRAVEL.** At 8 % - the angle a turning rotor
/// holds down to (`dimmer::HOLDS_AT`) - the measured curve gives `0.186·8^1.92 = 10 Pa`, so the steepness
/// there is `1.92·10/8`. The curve does not go flatter than that anywhere the regulator works, so a smaller
/// number coming out of the live `pa` and angle is not a flatter fan: it is noise at small travel, or air
/// going somewhere it should not - a damper shut, a tube off the port.
///
/// Without the floor those two cases divide a whole error by almost nothing, and the regulator spends its
/// entire rate limit every second chasing a draught that is not coming. It also closes the arithmetic: a
/// standing fan reads no pressure at no angle, and `0/0` would put a NaN into the angle - which is not a
/// number a triac can be asked for and not a state any later comparison would catch.
pub const SLOPE_LEAST_PA_PER_PERCENT: f32 = 2.4;

/// The longest the coming-down off a kick may last before the regulator takes the fan whatever the draught is
/// doing.
///
/// Forty-five seconds. At the tract's own pace a descent from the kick's angle to the floor is about twenty,
/// and from a purge's hundred per cent about seventy-five - this sits above the first and deliberately below
/// the second, because a descent that long is not a descent any more and holding a third of the mains on a fan
/// is not something to leave running.
pub const SPINNING_MOST_S: f32 = 45.0;

/// How still the draught has to be for the angle and the pressure to be talking about the same moment.
///
/// Three pascals - the breathing of the air itself (at 12 Pa the reading wandered between 9 and
/// 15). Asking for less would be asking the noise to stop.
pub const STEADY_PA: f32 = 3.0;

/// How long it has to be that still before the pair is believed, in seconds.
///
/// **TEN, AND IT IS THE TRACT'S OWN NUMBER.** A step of the angle is 95 % answered in ten seconds
/// (`tools/tract_lag_probe.py`, 21.09). So an error that has sat inside the noise for ten seconds is an error
/// about the angle standing NOW, not about the one from before the last change - which is the whole
/// difference between a fact about the fan and a snapshot of a transient.
pub const STEADY_S: f32 = 10.0;

/// The pair the board starts life with: what a fan of this kind held, measured on the burner 21.09.
///
/// **A FAN THAT HAS NEVER HELD ANYTHING LEAVES NOTHING TO REMEMBER, and the first start still has to go
/// somewhere.** 25 % of travel held 74 Pa on the owner's fan, and until this board's own fan has held a
/// setpoint once, that is the shape it reasons with. It is somebody else's fan, and [`Holding::knows`]
/// answers None while it is in use so that the board can say as much out loud rather than pass it off as
/// knowledge. The first time this fan holds a setpoint, the pair becomes its own.
pub const A_FAN_LIKE_THIS: (f32, f32) = (74.0, 25.0);

/// What the regulator is in the MIDDLE of: the manoeuvre, and nothing about the fan itself.
///
/// **IT IS ONE THING SO THAT HANDING THE FAN OVER CANNOT FORGET A PIECE OF IT.** Before this, the four fields
/// below sat loose in [`Holding`] and [`Holding::handed_over`] named them one by one - which worked exactly
/// until the next temporary field was added. It happened twice: first a remembered descent outlived a
/// handover and took the angle DOWN while the setpoint asked for more air; then the count of steady seconds
/// outlived one, and a pair the fan never proved was written after a single second. Both times the fix was
/// "add the name to the list", and both times the list was what failed.
///
/// Now the list is the type. A new field of the manoeuvre lands here, and `Manoeuvre::default()` drops it
/// along with the rest - by construction rather than by the author remembering.
///
/// **What is NOT here is what the fan proved about itself:** that it turns, the floor it needs, the pair it
/// held. A handover ends a manoeuvre, not the machine.
#[derive(Default)]
struct Manoeuvre {
    /// The error of the previous step; None before the first one, and then the proportional part contributes
    /// nothing - there is no change in an error that has only been seen once.
    was: Option<f32>,
    /// How long the present kick has been going, in seconds; None - not kicking.
    kicking: Option<f32>,
    /// How long the fan has been coming down off its kick, in seconds, and whether the draught has yet gone
    /// OVER the setpoint; None - not coming down off a kick.
    spinning: Option<(f32, bool)>,
    /// How long the error has sat inside the noise, in seconds - the counter that decides when a pair about
    /// the fan may be taken.
    steady: f32,
}

/// The regulator, holding one value.
pub struct Holding {
    /// The angle as it stands, in percent.
    angle: f32,
    /// What it is in the middle of. Dropped whole whenever the fan is handed over - see [`Manoeuvre`].
    doing: Manoeuvre,
    /// Whether the fan has PROVED it turns, by making a draught. Not an assumption and not a command: the board
    /// asks the pressure.
    turning: bool,
    /// Kicks given since the fan was last known to be standing.
    ///
    /// **NOT PART OF THE MANOEUVRE, AND THE DIFFERENCE IS DELIBERATE.** It counts a run of attempts to get
    /// this rotor moving, and what ends that run is the fan STANDING - `stood` clears it - not somebody
    /// taking the knob for a moment. Handed back after a hand or a purge, a fan that has already refused two
    /// kicks has still refused them.
    kicks: u32,
    /// The draught this fan was last seen HOLDING, and the angle that held it. None - it never has yet, and
    /// the board reasons with [`A_FAN_LIKE_THIS`] until it does.
    ///
    /// **THE SAME KIND OF THING AS `floor`: a fact the fan proved about itself.** The angle that holds a
    /// setpoint cannot be written into a setting - it moves with the chimney, the weather and how warm the
    /// motor is - and it cannot be hunted for during a start either, because the draught answers the angle
    /// four seconds late and the whole descent off a kick lasts three to five. The one moment it can be known
    /// is while the fan is actually holding: then the angle and the pressure are about the same moment.
    ///
    /// It outlives a stop, a purge and a hand on the knob, for the same reason the floor does: what is being
    /// remembered is the machine, not the manoeuvre.
    holding: Option<(f32, f32)>,
    /// The lowest angle this fan has been seen to KEEP TURNING at, and how long it has held above it.
    ///
    /// **THE NUMBER NOBODY CAN WRITE DOWN.** The angle a turning rotor holds at is not the angle a standing one
    /// breaks away at, and it moves: cold it wants more air, and as the motor warms it holds lower and lower
    /// (observed on 20.09 and 21.09). A setting cannot carry that - whatever a person types is right for one
    /// temperature - so the regulator finds it by trying, and lets it slide back down as the fan warms.
    floor: Option<(f32, f32)>,
    /// The lowest angle this fan was still PROVING it turns at, since the last time it proved it.
    ///
    /// **IT IS NOT THE ANGLE THE STALL IS NOTICED AT, AND THE TWO ARE SECONDS APART.** The draught empties at
    /// the tract's own pace - 63 % of a step down in three seconds, measured on the board - so the rotor is
    /// already standing while the pressure is still falling through the threshold. All those seconds the
    /// regulator sees a growing error and pushes the angle UP: traced on the model, the rotor fell out at
    /// 12.00 % and the stall was noticed at 13.91. A floor built on the second number is two percent of travel
    /// above the fact the fan proved, and it is the fan's fact we mean to keep.
    lowest_turning: Option<f32>,
    /// Whether the stall now under way has already been written down.
    ///
    /// **IT IS NOT `turning`, AND THAT DISTINCTION IS THE WHOLE OF THIS FIELD.** Clearing `turning` would fire
    /// the block once, correctly - and it would also tell the rest of the regulator that the fan is STANDING,
    /// which sends it to kick a rotor that may still be spinning down. What a fallen fan gets is decided
    /// elsewhere and deliberately: the tract empties, `Under` arrives, `stood` runs, and only then a kick.
    /// This flag says one thing only - the fact has been recorded, do not record it again.
    stalled: bool,
    /// What the last step made of the second it regulated; None - it did not regulate it. See [`Parts`].
    parts: Option<Parts>,
}

/// What the regulator made of one second it regulated, for a person watching it tune: the miss,
/// and the two halves of the move it made out of it.
///
/// **A RECORD OF THE ARITHMETIC AND NOTHING ELSE**: the step computes these numbers anyway, and keeping them changes
/// no angle. The halves are percent of angle for that second, BEFORE the move is cut to `step` and the angle to its
/// limits - so a person sees what the regulator wanted as well as what it was let do; `i` is the one after the
/// anti-windup, the one the move was made of.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Parts {
    /// What was asked less what came out, in pascals. **ITS SIGN IS THE SET OF GAINS**: at or over nought the `up`
    /// set moved the angle, under nought the `down` one.
    pub miss_pa: f32,
    /// The proportional half, from the change of the miss since the second before.
    pub p: f32,
    /// The integral half, from the miss itself.
    pub i: f32,
}

impl Parts {
    /// The three points a curve draws of a second - the miss, P, I - and NaN for all three in a second the regulator
    /// did not regulate: the curve breaks there, where a zero would say "no miss and no move", which did not happen.
    pub fn points(parts: Option<Parts>) -> [f32; 3] {
        parts.map_or([f32::NAN; 3], |p| [p.miss_pa, p.p, p.i])
    }
}

impl Holding {
    /// Starts at the angle the fan is ALREADY held at.
    pub fn from(angle: f32) -> Self {
        Self { angle, doing: Manoeuvre::default(), turning: false, kicks: 0, holding: None, floor: None, lowest_turning: None, stalled: false, parts: None }
    }

    /// What THIS fan was last seen holding: the draught, and the angle that held it.
    ///
    /// None means the board is still reasoning with [`A_FAN_LIKE_THIS`] - somebody else's fan - and whoever
    /// shows a person what the board is doing should say so rather than let it pass for knowledge.
    pub fn knows(&self) -> Option<(f32, f32)> {
        self.holding
    }

    /// Hands the board a pair it did not learn in this session - what it held before it was last stopped.
    pub fn was_holding(&mut self, pa: f32, angle: f32) {
        if pa > 0.0 && angle > 0.0 {
            self.holding = Some((pa, angle));
        }
    }

    /// The angle that holds WANT on the fan the board reasons with, in percent.
    ///
    /// The pair says this fan makes `pa` at `angle`; the curve says pressure goes as `angle^n`; so the angle
    /// for any other pressure is `angle·(want/pa)^(1/n)`. **NO DRAUGHT IS READ HERE AT ALL** - that is the
    /// point. During a start the draught is four seconds behind the angle, and every attempt to steer by it
    /// traded one fault for another.
    fn angle_for(&self, want: f32) -> f32 {
        let (pa, angle) = self.holding.unwrap_or(A_FAN_LIKE_THIS);
        angle * (want / pa).powf(1.0 / curve_power(angle))
    }

    /// The angle it holds now, in percent.
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// Whether the fan has proved it turns.
    pub fn turning(&self) -> bool {
        self.turning
    }

    /// What the last step made of its second, when it regulated it.
    pub fn parts(&self) -> Option<Parts> {
        self.parts
    }

    /// One step: what the fan is asked to do, what the sensor says, how long since the last step.
    pub fn step(&mut self, asked: Asked, said: Reading, secs: f32, t: &Tuning) -> Held {
        // a second is told about only if this step regulates it
        self.parts = None;
        // What the reading is worth as a pressure, and whether it proves the fan turns. `Over` is a pressure
        // too - a one-sided one: it says "more than this sensor can name", which is certainly above any
        // setpoint the regulator holds, and certainly a turning fan.
        let pa = match said {
            Reading::Is(pa) => Some(pa),
            Reading::Over => Some(draught::FULL_PA as f32),
            Reading::Under | Reading::Missing | Reading::Unconfigured => None,
        };
        // **THE FAN JUST FELL OUT, AND THE ANGLE IT FELL OUT AT IS THE FACT WORTH KEEPING.** It was turning a
        // second ago and it is not now, so this angle does not hold THIS fan at THIS temperature - and the one
        // above it is the lowest that is known to. Written down here, the regulator never walks down into the
        // same stall twice; `warmed_down` lets it back below as the motor heats.
        // **THE LOWEST ANGLE THE FAN WAS STILL PROVING ITSELF AT**, kept while the draught says it turns. It is
        // the angle the rotor actually fell out at; the one the stall is noticed at is seconds later and higher.
        // **NOTHING IS REMEMBERED ABOUT AN ANGLE THE REGULATOR IS NOT COMMANDING.** Under a hand, a purge or a
        // stop the fan is at whatever somebody else set it to, and `self.angle` is merely what the regulator
        // left behind - so a stall noticed then writes a floor from a number that is about nobody, and the
        // floor outlives the handover. It is the same rule the manoeuvre lives by: what the board keeps has to
        // be a fact about the machine, and an angle it did not choose is not one.
        let commanding = matches!(asked, Asked::Hold(want) if want > 0.0);
        // ONLY AN ANGLE THE REGULATOR ACTUALLY CHOSE COUNTS. Before the fan has proved it turns the angle is
        // whatever a kick or a standstill left behind - nought at the first second, a third of the mains during
        // one - and the lowest of those is a fact about the manoeuvre, not about the motor.
        if commanding
            && pa.is_some_and(|pa| pa >= TURNS_ABOVE_PA)
            && self.turning
            && self.doing.kicking.is_none()
            && self.doing.spinning.is_none()
        {
            self.lowest_turning = Some(self.lowest_turning.map_or(self.angle, |low: f32| low.min(self.angle)));
        }
        if commanding && self.turning && !self.stalled && pa.is_some_and(|pa| pa < TURNS_ABOVE_PA) {
            // **ONCE PER STALL, AND THAT IS WHAT CLEARING `turning` HERE IS FOR.** The comment above describes a
            // TRANSITION - it was turning and now it is not - and the condition tests a LEVEL, so while the
            // draught stayed below the threshold this fired every second and took the floor up a percent each
            // time: eight seconds of an emptying tract used to add eight percent to a fact worth one step. The
            // fan proves it turns again by making a draught, and the block below sets the flag back.
            let fell_at = self.lowest_turning.unwrap_or(self.angle);
            let found = (fell_at + STEP_ABOVE_A_STALL).max(self.floor.map_or(0.0, |(at, _)| at));
            self.stalled = true;
            self.lowest_turning = None;
            self.floor = Some((found, 0.0));
            // and the fan goes back up to it NOW rather than at the regulator's pace: every second under the
            // holding angle is a second the rotor is losing what the kick gave it
            self.angle = self.angle.max(found);
            self.doing.spinning = None;
            self.doing.was = None;
        }
        if pa.is_some_and(|pa| pa >= TURNS_ABOVE_PA) {
            // **A KICK THAT TOOK IS NOT A FAN THAT IS RUNNING.** The rotor has been moving for one second and
            // is nowhere near its speed; the angle it will HOLD at is far below the angle it needed to break
            // away, but it only holds there once it is up to speed - and warm, it holds lower still (observed on
            // 20.09). Dropped to the floor in the second the draught appeared, a cold rotor falls straight back
            // out: measured on the board 21.09, the angle went 33 → 8 and the pressure 25 → 1, and then the
            // whole kick happened again. Warm it crawled back; cold it hammered.
            //
            // So a kick that took hands over to SPINNING UP, which holds the kick's own angle while the draught
            // is still climbing. Nothing is regulated during it - there is nothing to regulate yet.
            //
            // Only out of a KICK. A regulator switched on while the fan already blows keeps the angle it was
            // handed - that is what `Holding::from` is for, and slamming it to anything would jerk the fan.
            if self.doing.kicking.is_some() {
                self.doing.spinning = Some((0.0, false));
                self.doing.was = None;
            }
            self.turning = true;
            // the fan is making a draught again, so the NEXT time it stops making one is a new fact
            self.stalled = false;
            self.doing.kicking = None;
            self.kicks = 0;
        }

        match asked {
            // NOTHING IS WRITTEN, not even a zero: the angle belongs to the person holding it. The regulator
            // forgets what it knew, so that taking it back is a start and not a continuation of an old error.
            Asked::ByHand => {
                self.handed_over();
                Held::Alone
            }
            Asked::Stop => {
                self.stood();
                Held::At(self.angle)
            }
            Asked::Purge => {
                // full air, and everything the regulator was in the middle of is forgotten: carried across a
                // purge, it throws the first step of the holding that follows
                self.handed_over();
                self.angle = 100.0;
                Held::At(self.angle)
            }
            // A setpoint of no pascals is not a pressure to hold. Chasing it would leave the motor humming at
            // the floor of the angle, which is the one thing `Stop` exists to avoid.
            Asked::Hold(want) if want <= 0.0 => {
                self.stood();
                Held::At(self.angle)
            }
            Asked::Hold(want) => match self.turning {
                false => self.break_away(secs),
                true => match pa {
                    Some(pa) if self.doing.spinning.is_some() => self.easing(want, pa, secs, t),
                    Some(pa) => self.moved(want, pa, secs, t),
                    // it WAS turning and now there is no number: that is a fault, and holding the last angle
                    // blind is the dangerous answer
                    None => {
                        self.stood();
                        Held::Blind(match said {
                            Reading::Under => Blind::Under,
                            Reading::Missing => Blind::Missing,
                            _ => Blind::Unconfigured,
                        })
                    }
                },
            },
        }
    }

    /// The decision of the last step did not reach the triac, so the regulator forgets what it knew.
    ///
    /// **THE ANGLE IS AN INTENTION AND THE INTEGRAL IS BUILT ON IT.** Left running while the gate refuses, the
    /// loop keeps climbing on an error it can do nothing about - there is no air, so the error never closes -
    /// and the angle walks to the ceiling in silence. The moment the gate comes back, the fan is commanded to
    /// an angle nobody chose, cold. The clamping in `moved` does not help: it only stops the integral running
    /// FURTHER out once the angle is already on a limit, and the walk to that limit is the damage.
    ///
    /// The same answer as `Blind`, and for the same reason: what cannot be carried out cannot be continued
    /// from. Taking the fan back afterwards is a start, with its kick, and not a continuation of an old error.
    pub fn not_carried_out(&mut self) {
        self.stood();
    }

    /// Coming down off the kick's angle ONE STEP AT A TIME, instead of dropping to the floor in one second.
    ///
    /// **BOTH ENDS OF THIS WERE TRIED AND BOTH ARE WRONG.** Dropped to the floor the second the draught
    /// appeared, a rotor that has been turning for one second falls straight back out - measured on the board
    /// 21.09: the angle went 33 → 8, the pressure 25 → 1, and the whole kick happened again; warm it crawled
    /// back, cold it hammered. Left at the kick's angle instead, the loop overshoots badly, because the
    /// pressure has not caught up with the angle yet: on the model a start onto 51 Pa reached 134 Pa.
    ///
    /// What is wrong in both is the SPEED of the change, not where it ends. Walking down at the regulator's own
    /// step gives the rotor the seconds it needs to come up, and takes the angle off fast enough that the
    /// draught never runs away. It ends when the draught has reached what was asked - from there the regulator
    /// has something to regulate - or at the floor, whichever comes first.
    fn easing(&mut self, want: f32, pa: f32, secs: f32, t: &Tuning) -> Held {
        let (been, over) = self.doing.spinning.unwrap_or((0.0, false));
        let over = over || pa >= want;
        // **THE DESCENT ENDS WHERE THE FAN IS KNOWN TO HOLD, NOT AT THE FLOOR OF THE SETTING.** Left to fall
        // to the floor, the angle overshoots the mark by nine percent of travel and the draught follows it
        // down: measured 22.09, a start onto 31 Pa fell to 11 Pa with the angle sitting at 9 %, and the
        // regulator then spent over a minute crawling back to the 17 % that holds it. The angle that holds is
        // not something to be discovered during the descent - by the time the draught could say, the descent
        // is over. It is [`Holding::angle_for`], and it is known before the kick is even given.
        let floor = self.floor_now(t).max(self.angle_for(want));
        // **THE EASING ENDS WHEN THE DRAUGHT HAS COME DOWN TO THE SETPOINT, NOT WHEN IT FIRST REACHES IT.**
        // Handing over the moment the draught passed the setpoint left the regulator to take the rest off at
        // its own gentle pace, and the air kept climbing meanwhile: measured on the board 21.09, a start onto
        // 30 Pa reached 87 Pa and stayed over the setpoint for a minute. The angle of a kick is a third of the
        // mains and the tract answers it ten seconds later - what is above the setpoint has to come off at the
        // speed it went on.
        if (over && pa <= want) || self.angle <= floor + 0.001 || been + secs >= SPINNING_MOST_S {
            self.doing.spinning = None;
            self.doing.was = None;
            return Held::At(self.angle);
        }
        self.doing.spinning = Some((been + secs, over));
        // **THE ANGLE IS KNOWN, SO IT IS SET RATHER THAN APPROACHED.** Walking down to it spends three to five
        // seconds at an angle far above the one that holds - and the tract turns those seconds into pressure:
        // measured 22.09, a start onto 31 Pa reached 57 Pa while the angle was still on its way down. There is
        // nothing to discover on the way, because the destination came from the pair the fan taught the board,
        // not from the draught being read now.
        if self.angle > floor {
            self.angle = floor;
            return Held::At(self.angle);
        }
        // **THE DESCENT GOES AT THE TRACT'S PACE, NOT THE REGULATOR'S.** `t.step` is the step of REGULATING,
        // where the error is small and there is another correction a second later; used to come down off a
        // kick it takes the angle from 33 % to the floor in five seconds, while the pressure it is supposed to
        // be watching needs four just to show 63 % of what the last change did. Measured on the board 21.09:
        // the angle went 33 → 28 → 13 → 8 in four seconds and the draught was still CLIMBING - 1 → 37 → 56 →
        // 44 - so whatever condition ends the descent, it is asked after the angle is already on the floor.
        //
        // Spread over the tract's own time constant, each step has been answered before the next one is made,
        // and the condition that ends the descent is asked about a pressure that means something.
        //
        // **EXCEPT WHILE THERE IS TOO MUCH AIR, WHERE THE PACE IS THE OTHER WAY ROUND.** Under the setpoint,
        // hurrying is what drops the rotor, so the descent waits for the tract. Over it, waiting is what feeds
        // a fire air nobody asked for, and the rotor is in no danger at all - it is turning on more angle than
        // it needs. So above the setpoint the angle comes off at the regulator's full step, and the slow pace
        // begins where the risk does.
        let pace = match pa > want {
            true => t.step * secs,
            false => t.step * secs / TRACT_ANSWERS_S,
        };
        self.angle = (self.angle - pace).max(floor);
        Held::At(self.angle)
    }

    /// How steeply the draught answers the angle RIGHT HERE, in pascals per percent.
    ///
    /// **THIS IS WHAT MAKES ONE SET OF GAINS RIGHT ON THE WHOLE TRAVEL.** The fan is not a straight line:
    /// measured on the board, a percent of angle is worth 4.9 Pa around 20 % of travel and 6.9 Pa around 42 %.
    /// Gains picked at one end are either sluggish or twitchy at the other, and no amount of tuning fixes
    /// that - it is the shape of the fan, not the setting. Divided by the steepness here, the error stops
    /// being pascals and becomes percents of angle, and the fan looks the same everywhere.
    ///
    /// Taken from the live `pa` and angle rather than from a fitted curve: `n·pa/angle` is the derivative of
    /// `k·angle^n`, and it needs neither a logarithm nor a stored `k`. The power `n` itself is not one number
    /// on this fan - see [`curve_power`]. At the bottom of the travel both `pa` and the angle are small and
    /// their ratio is noise, so the steepness is floored - see [`SLOPE_LEAST_PA_PER_PERCENT`].
    pub fn slope(pa: f32, angle: f32) -> f32 {
        match angle > 0.0 {
            true => (curve_power(angle) * pa / angle).max(SLOPE_LEAST_PA_PER_PERCENT),
            false => SLOPE_LEAST_PA_PER_PERCENT,
        }
    }

    /// The lowest angle the regulator may command now: what a person set, or higher if this fan has shown it
    /// needs more air to keep turning.
    fn floor_now(&self, t: &Tuning) -> f32 {
        self.floor.map_or(t.min, |(at, _)| at.max(t.min).min(t.max))
    }

    /// A motor that has been holding comfortably above its found floor for a while is warmer than it was, so
    /// the floor comes down a step and the regulator is free to try lower.
    ///
    /// **IT ONLY EVER SLIDES DOWN.** Going up is the stall's job, and it is instant - that is a fact the fan
    /// just proved. Coming down is a guess, so it is made slowly and one step at a time, and if it was wrong
    /// the next stall puts the floor back where it was.
    fn warmed_down(&mut self, secs: f32, t: &Tuning) {
        let Some((at, held)) = self.floor else { return };
        // only counts while the regulator is actually sitting near the floor: an angle far above it says
        // nothing about whether the fan could hold lower
        if self.angle > at + t.step {
            self.floor = Some((at, 0.0));
            return;
        }
        let held = held + secs;
        match held >= FLOOR_SLIDES_AFTER_S {
            true => {
                let lower = at - t.step;
                self.floor = match lower <= t.min {
                    true => None,
                    false => Some((lower, 0.0)),
                };
            }
            false => self.floor = Some((at, held)),
        }
    }

    /// The fan is stopped and knows nothing about itself again.
    fn stood(&mut self) {
        self.angle = 0.0;
        self.handed_over();
        self.turning = false;
        self.lowest_turning = None;
        self.stalled = false;
        self.kicks = 0;
    }

    /// The regulator is giving the fan to somebody else - a hand, a purge, a stop - and drops everything it
    /// was in the MIDDLE of.
    ///
    /// **A HALF-FINISHED START MUST NOT OUTLIVE THE START.** Three things here are temporary by nature: the
    /// error of the last step, a kick being timed, and a descent off a kick. Whatever comes next begins its
    /// own way - a person's hand, full air, a fresh setpoint - and none of them continues what was happening a
    /// moment ago. Left behind, the descent was the loud one: handed the fan back with a setpoint asking for
    /// MORE air, the regulator went on taking the angle DOWN, because the start it had been in twenty seconds
    /// earlier was still in its head. The purge was the same defect one door along - it sets a hundred per
    /// cent, and the remembered descent took the angle off it in the next second.
    ///
    /// What is NOT dropped is what the fan has PROVED about itself - that it turns, and the floor it needs -
    /// because those are facts about the machine rather than about the manoeuvre. `stood` drops those too,
    /// and only it: a fan that has stopped has stopped proving anything.
    fn handed_over(&mut self) {
        self.doing = Manoeuvre::default();
    }

    /// Getting a standing rotor to move. There is no draught to read yet BY CONSTRUCTION - the sensor puts its
    /// own zero at half a volt and a standing fan sits under it - so this phase is timed, not measured, and what
    /// measures it is the draught that appears when the kick works.
    fn break_away(&mut self, secs: f32) -> Held {
        // THE AIR COMES FIRST AND THE VERDICT ON THE STEP AFTER. Measured on the board 21.09: judged in the
        // same step that gave it, the kick was condemned having blown for two seconds of the three it was
        // allowed, and the board said the fan was not turning while it was still spinning up. A fan is judged
        // on what it did with the air it was given.
        let Some(been) = self.doing.kicking else {
            self.doing.kicking = Some(0.0);
            self.angle = KICK_PERCENT;
            return Held::At(self.angle);
        };
        let been = been + secs;
        if been < KICK_SECONDS {
            self.doing.kicking = Some(been);
            self.angle = KICK_PERCENT;
            return Held::At(self.angle);
        }
        // the kick has had its second of air and the draught has not appeared: it did not move
        self.kicks += 1;
        if self.kicks >= KICKS {
            self.angle = 0.0;
            self.doing.kicking = None;
            return Held::Blind(Blind::NeverStarted);
        }
        self.doing.kicking = Some(0.0);
        self.angle = KICK_PERCENT;
        Held::At(self.angle)
    }

    /// The step itself: an increment of angle out of the error.
    fn moved(&mut self, want: f32, pa: f32, secs: f32, t: &Tuning) -> Held {
        let e = want - pa;
        let g = match e >= 0.0 {
            true => t.up,
            false => t.down,
        };
        let near = match e.abs() < t.band_pa {
            true => t.band_share,
            false => 1.0,
        };
        let (kp, ki) = (near * g.kp_milli as f32 / 1000.0, near * g.ki_milli as f32 / 1000.0);
        // **THE REMEMBERED ERROR STAYS IN PASCALS AND BOTH HALVES ARE DIVIDED BY THE STEEPNESS OF THIS STEP.**
        // Normalising the error BEFORE storing it would compare a miss measured against last second's
        // steepness with one measured against this second's, and the draught breathes about 5 % - so the
        // proportional half would be reading the noise of the steepness rather than the change of the error.
        // Divided here, the whole increment is simply the pascal one scaled by one number, which is what
        // "the fan looks the same all the way" means in arithmetic.
        let slope = Self::slope(pa, self.angle);
        // no change in an error seen once, so the first step is the integral alone
        let p = kp * (e - self.doing.was.unwrap_or(e)) / slope;
        let mut i = ki * e * secs / slope;
        // CLAMPING, and it is the cheap half of anti-windup that is enough here: the integral does not
        // accumulate FURTHER OUT while the angle already sits on that limit. Without it, ten minutes against
        // the ceiling hold the fan at full long after the setpoint has been lowered, and a person watches
        // nothing happen.
        let edge = 0.001;
        if (self.angle >= t.max - edge && i > 0.0) || (self.angle <= t.min + edge && i < 0.0) {
            i = 0.0;
        }
        self.parts = Some(Parts { miss_pa: e, p, i });
        let moved = (p + i).clamp(-t.step, t.step);
        // THE FLOOR IS THIS FAN'S, not the setting's: `floor_now` is what a person typed, raised to whatever
        // this motor has shown it needs to keep turning at the temperature it is at now.
        let held_at = self.angle;
        self.angle = (self.angle + moved).clamp(self.floor_now(t), t.max);
        // **THE ONE MOMENT THE ANGLE AND THE PRESSURE ARE ABOUT THE SAME THING.** The error is inside the
        // noise and has stopped changing, so the draught being read is the answer to the angle standing now
        // rather than to one from several seconds ago. That is when a pair can be taken, and the angle in it
        // is the one that was holding - before this step moved it.
        match e.abs() <= STEADY_PA && (e - self.doing.was.unwrap_or(e)).abs() <= STEADY_PA {
            true => {
                self.doing.steady += secs;
                if self.doing.steady >= STEADY_S {
                    self.holding = Some((pa, held_at));
                }
            }
            false => self.doing.steady = 0.0,
        }
        self.doing.was = Some(e);
        self.warmed_down(secs, t);
        Held::At(self.angle)
    }
}

/// What the board says when it is holding nothing: the fault, and what a person does about it.
///
/// SHORT WORDS AND NO HISTORY. This is a row in a card on a phone, and it says the state now and the next
/// action - never what the board used to do (the domain's charter).
pub fn words(why: Blind) -> String {
    match why {
        Blind::Under => format!(
            "fan stopped: the draught sensor reads under {} mV, its own zero - check the tube's port",
            draught::ZERO_MV
        ),
        Blind::Missing => format!(
            "fan stopped: no draught sensor - under {} mV on its pin; check its 12 V",
            draught::MISSING_BELOW_MV
        ),
        Blind::Unconfigured => {
            "fan stopped: the draught divider is not a divider - fill both resistances on its card".to_string()
        }
        Blind::NeverStarted => format!(
            "fan stopped: kicked {KICKS} times at {KICK_PERCENT:.0} % and made no draught - it is not turning, \
             check the motor and the triac"
        ),
    }
}

/// What became of one decision when it reached the triac.
///
/// **A REGULATOR THAT REPORTS HOLDING WITHOUT HOLDING IS WORSE THAN NO REGULATOR.** The angle the loop decides
/// on is an intention; whether the gate took it is a separate fact, and the board knows it. Swallowed - which is
/// what `holding_round` did until 21.09, writing a `warn` and then publishing the angle as held - the card, the
/// log and the curve all said the draught was being held while the triac could be anywhere. On a live fire that
/// is fuel going into a pot nobody is blowing.
#[derive(Clone, PartialEq, Debug)]
pub enum Carried {
    /// The gate took the angle.
    Out,
    /// The gate refused the angle and the fan was stopped instead: the triac is at zero.
    Stopped { angle: u32, why: String },
    /// The gate refused the angle AND refused to stop. What the triac holds is not known - not the old angle,
    /// not zero, because a refusal can land between the two comparators.
    Lost { angle: u32, why: String, stop_why: String },
}

/// Carries one decision to the gate, and says what became of it.
///
/// **A REFUSED ANGLE IS FOLLOWED BY A STOP, ALWAYS.** Not "keep the last angle and try again next second": the
/// last angle was set for a draught that is no longer what is wanted, and a second of it is a second of air the
/// fire is not getting or getting too much of. Zero is the answer the gate is least able to refuse - it needs
/// neither a measured half cycle nor a built gate (`fan::hold`) - so it is the one worth asking for.
///
/// GATE IS HANDED IN rather than reached for, so this whole decision is judged on a computer: the board's own
/// gate is a triac and cannot be asked to refuse on purpose.
pub fn carry(angle: u32, gate: &mut dyn FnMut(u32) -> Result<String, String>) -> Carried {
    let Err(why) = gate(angle) else { return Carried::Out };
    match gate(0) {
        Ok(_) => Carried::Stopped { angle, why },
        Err(stop_why) => Carried::Lost { angle, why, stop_why },
    }
}

/// What one round of the regulator came to: what it decided, the angle it asked the gate for, and what became of it.
#[derive(Clone, PartialEq, Debug)]
pub struct Round {
    pub held: Held,
    /// The whole percent the gate was asked for; None - nothing was commanded, because a person holds the angle by hand.
    pub commanded: Option<u32>,
    pub carried: Carried,
}

/// One round: the regulator decides on what the sensor said, the decision is carried to the gate, and the regulator is told when it did not get through. What the rest of a round does - the curves, the card, the log - is the board's.
///
/// THE SAME ROUND ON THE BOARD AND ON THE STAND: the board's gate is a triac and cannot be asked to refuse on purpose, so it is handed in, and a stand that runs this function runs the decision the board runs.
pub fn round(holding: &mut Holding, asked: Asked, said: Reading, secs: f32, t: &Tuning, gate: &mut dyn FnMut(u32) -> Result<String, String>) -> Round {
    let held = holding.step(asked, said, secs, t);
    let commanded = match held {
        Held::At(angle) => Some(angle.round() as u32),
        // it cannot read the draught: the fan stops. Holding the last angle blind is the dangerous answer - fuel into a pot the board believes is being blown.
        Held::Blind(_) => Some(0),
        // A PERSON IS HOLDING THE ANGLE BY HAND AND NOTHING IS WRITTEN. Not even a zero: writing one once a second is what took the hand-set angle away before the fan had spun up (measured on the board 21.09).
        Held::Alone => None,
    };
    let carried = match commanded {
        Some(angle) => carry(angle, gate),
        // nothing was commanded, so nothing could refuse
        None => Carried::Out,
    };
    // so that it stops climbing on an error it can do nothing about
    if !matches!(carried, Carried::Out) {
        holding.not_carried_out();
    }
    Round { held, commanded, carried }
}

/// What the board says when its own command did not get through: the state now and the next action.
///
/// The same shape as [`words`] and for the same reason - this is a row in a card on a phone. A person reading
/// it has to learn TWO things a log line would not tell them: that the draught is no longer being held, and
/// which of the two it is - a fan standing at zero, or a triac nobody can account for.
pub fn gate_words(carried: &Carried) -> Option<String> {
    match carried {
        Carried::Out => None,
        Carried::Stopped { angle, why } => Some(format!(
            "fan stopped: the gate refused {angle} % and the fan was put to 0 - {why}"
        )),
        Carried::Lost { angle, why, stop_why } => Some(format!(
            "THE FAN IS NOT UNDER CONTROL: the gate refused {angle} % ({why}) and refused to stop ({stop_why}) - \
             cut the fan's power and check the triac"
        )),
    }
}

/// How long [`answer_to_a_jump`] follows the draught after the jump, in seconds: four times the fifteen the regulator
/// is held to, so an answer that never comes back shows as a line that never comes back.
pub const ANSWER_S: usize = 60;

/// What the regulator makes of a jump in the tract: when the draught is back near its setpoint, and how far it dips
/// under it on the way.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Jump {
    /// The second from which the draught stays within the band of the setpoint (`Tuning::band_pa`, or
    /// [`STEADY_PA`] with no band) for [`STEADY_S`]; None - not within [`ANSWER_S`].
    pub back_s: Option<u32>,
    /// How far under the setpoint the draught went on its way back, in pascals; 0 - it never crossed it.
    pub dip_pa: f32,
}

/// What THIS regulator, on tuning T, does with a jump of JUMP_PA in the tract while it holds PA at ANGLE: the answer a
/// fan card gives a person under its knobs.
///
/// **THE SAME `Holding::step` THE BOARD RUNS, ON A TRACT DRAWN THROUGH THE POINT GIVEN.** The fan's curve is the one the
/// regulator reasons with, `pa·(a/angle)^n` with `n` of [`curve_power`]; the tract answers in [`TRACT_ANSWERS_S`]; the
/// gate is given a whole percent, as [`carry`] is. At the first second the tract grows steeper, so the same angle
/// makes JUMP_PA more - the igniter going off did that on 26.09 - and the regulator takes it back. EACH is told the
/// pressure and the angle of every second, [`ANSWER_S`] of them, for a curve; nothing is kept here.
///
/// A point that is no point - no pressure, no angle, not a number - answers None and draws nothing: a fan that holds
/// nothing has no answer to give, and a made-up one would be read as the board's word.
pub fn answer_to_a_jump(t: &Tuning, pa: f32, angle: f32, jump_pa: f32, each: &mut dyn FnMut(f32, f32)) -> Jump {
    let nothing = Jump { back_s: None, dip_pa: 0.0 };
    if !(pa > 0.0 && angle > 0.0 && jump_pa.is_finite()) {
        return nothing;
    }
    let n = curve_power(angle);
    let steeper = (pa + jump_pa) / pa;
    let makes = |a: f32| steeper * pa * (a.round().max(0.0) / angle).powf(n);
    let mut holding = Holding::from(angle);
    // a fan already holding PA: it proved it turns, and the manoeuvre is over
    holding.turning = true;
    let mut now = pa;
    let mut at = angle;
    let within = match t.band_pa > 0.0 {
        true => t.band_pa,
        false => STEADY_PA,
    };
    let (mut inside, mut back_s, mut dip_pa) = (0.0f32, None, 0.0f32);
    for second in 1..=ANSWER_S as u32 {
        now += (makes(at) - now) / TRACT_ANSWERS_S;
        if let Held::At(a) = holding.step(Asked::Hold(pa), draught::Reading::Is(now), 1.0, t) {
            at = a;
        }
        each(now, at);
        dip_pa = dip_pa.max(pa - now);
        inside = match (now - pa).abs() <= within {
            true => inside + 1.0,
            false => 0.0,
        };
        if back_s.is_none() && inside >= STEADY_S {
            back_s = Some(second + 1 - STEADY_S as u32);
        }
    }
    Jump { back_s, dip_pa }
}
