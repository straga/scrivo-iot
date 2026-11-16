//! A failure that repeats, said when it begins and then once in a while with its count.
//!
//! The board that cannot join its network tries every ~74 s, and each try wrote its line: 14 of them in the tail of the journal, all the same words. The first failure says what is wrong; the thousand
//! after it say that it still is. So the first is said at once, the rest are counted, and the line comes again when a period has run since the last - saying how many tries there have been and
//! how many of them went unsaid. A success ends the run: the next failure is the first of a new one.
//!
//! The seconds are the caller's (a counter that only goes forward); the words are the caller's too.

/// What a line about a repeated failure carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Said {
    /// How many times it has failed in this run, this one included.
    pub attempts: u32,
    /// How many of those went unsaid since the last line (the first line has none).
    pub unsaid: u32,
}

/// The count of one failure that repeats.
pub struct Repeats {
    period_s: u32,
    attempts: u32,
    unsaid: u32,
    /// When the last line was said, in the caller's seconds; None before the first failure of a run.
    said_at: Option<u64>,
}

impl Repeats {
    /// A failure said at most once in PERIOD_S seconds after its first.
    pub const fn new(period_s: u32) -> Self {
        Self { period_s, attempts: 0, unsaid: 0, said_at: None }
    }

    /// It failed again at NOW_S. -> the line to say, or None while the period since the last line has not run. A clock that has not moved forward since the last line says nothing: a line for a clock
    /// that went wrong is worse than a missing one.
    pub fn fail(&mut self, now_s: u64) -> Option<Said> {
        self.attempts += 1;
        match self.said_at {
            None => {
                self.said_at = Some(now_s);
                Some(Said { attempts: self.attempts, unsaid: 0 })
            }
            Some(at) if now_s > at && now_s - at >= u64::from(self.period_s) => {
                let said = Said { attempts: self.attempts, unsaid: self.unsaid };
                self.said_at = Some(now_s);
                self.unsaid = 0;
                Some(said)
            }
            Some(_) => {
                self.unsaid += 1;
                None
            }
        }
    }

    /// How many of this run's failures went unsaid since its last line.
    pub fn unsaid(&self) -> u32 {
        self.unsaid
    }

    /// How many times it has failed in this run: nought before the first failure and after a success.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// It worked: the run is over, and the next failure is the first of another.
    pub fn ok(&mut self) {
        self.attempts = 0;
        self.unsaid = 0;
        self.said_at = None;
    }
}

/// The count of one failure for each of a list - the networks a board may join: every one of them has its own tries and its own line, and a line about one carries nothing of the others. A join ends them all: the board is in a
/// network, and what failed before is history.
pub struct Each {
    runs: Vec<Repeats>,
}

impl Each {
    /// A list of COUNT, each said at most once in PERIOD_S seconds after its first failure.
    pub fn new(period_s: u32, count: usize) -> Self {
        Self { runs: (0..count).map(|_| Repeats::new(period_s)).collect() }
    }

    /// The one at WHICH failed again at NOW_S. -> the line to say for it; None while it is not due, and for a place the list does not have.
    pub fn fail(&mut self, which: usize, now_s: u64) -> Option<Said> {
        self.runs.get_mut(which)?.fail(now_s)
    }

    /// Whether none of them has failed in this run: a try that follows no failure is worth a line of its own.
    pub fn quiet(&self) -> bool {
        self.runs.iter().all(|run| run.attempts() == 0)
    }

    /// It joined: every run is over.
    pub fn ok(&mut self) {
        self.runs.iter_mut().for_each(Repeats::ok);
    }
}

/// The count of one failure for each of a set of keys met as they come - the access points a handshake breaks at, known by their BSSID only when one breaks. Every key has its own run and its own line; a join at one
/// ends the run of that one and leaves the others as they were. N keys are held: a key past them takes the place of the oldest, whose run is forgotten.
pub struct Keyed<K, const N: usize> {
    period_s: u32,
    runs: heapless::Vec<(K, Repeats), N>,
}

impl<K: Copy + PartialEq, const N: usize> Keyed<K, N> {
    /// Each key said at most once in PERIOD_S seconds after its first failure.
    pub const fn new(period_s: u32) -> Self {
        Self { period_s, runs: heapless::Vec::new() }
    }

    /// The key KEY broke again at NOW_S. -> the line to say for it; None while it is not due.
    pub fn fail(&mut self, key: K, now_s: u64) -> Option<Said> {
        let place = match self.runs.iter().position(|(k, _)| *k == key) {
            Some(place) => place,
            None => {
                if self.runs.is_full() {
                    self.runs.remove(0);
                }
                let _ = self.runs.push((key, Repeats::new(self.period_s)));
                self.runs.len() - 1
            }
        };
        self.runs[place].1.fail(now_s)
    }

    /// It joined at KEY: the run of that key is over, and the runs of the others go on.
    pub fn ok(&mut self, key: &K) {
        if let Some((_, run)) = self.runs.iter_mut().find(|(k, _)| k == key) {
            run.ok();
        }
    }

    /// Every run is over, for a join whose point is not known.
    pub fn ok_all(&mut self) {
        self.runs.iter_mut().for_each(|(_, run)| run.ok());
    }
}
