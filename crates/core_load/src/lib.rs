//! How busy each core of the board is: the share of a window that its idle task did not run.
//!
//! **TWO NUMBERS, NOT ONE.** The board has two cores and both work; one busy core under a free one is hidden by the mean
//! of the two, and the first finding of a fall was exactly that - a thread turning on the spot on a core while the other sat
//! free. What a core is busy with is not said here: a task waiting for the network is asleep and costs nothing, a task
//! spinning costs the whole core, and that is the difference this number is for.
//!
//! No hardware here: the firmware reads the clock and the idle tasks' counters (FreeRTOS run time stats, by the same
//! timer) and hands them over as a [`Reading`]; this does the arithmetic, so the part that has edge cases - the counters go
//! round every 71 minutes, the two are read a moment apart - is held on a computer.

/// The cores of the chips the boards use: the ESP32 and the ESP32-S3 have two.
pub const CORES: usize = 2;

/// How long a window is at least, in microseconds. Shorter, and a pass of the flash's owner that happened to be busy reads as the core being so.
pub const WINDOW_US: u32 = 5_000_000;

/// The counters at one moment, all in microseconds of the one timer, in 32 bits - which go round every 71 minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The clock the run time stats are counted by.
    pub wall_us: u32,
    /// What each core's idle task has run since the start.
    pub idle_us: [u32; CORES],
}

/// The share of the last whole window each core was not idle. Fed a reading at every pass; says a number once a window is over.
#[derive(Clone, Copy, Debug, Default)]
pub struct Load {
    base: Option<Reading>,
    said: Option<[u8; CORES]>,
}

impl Load {
    pub const fn new() -> Self {
        Load { base: None, said: None }
    }

    /// Takes the counters of now. The first is only the start of a window; a window is over when [`WINDOW_US`] of the clock has gone, and then it is
    /// counted from its first reading to this one - the readings between change nothing - and the next starts here.
    pub fn feed(&mut self, now: Reading) {
        let Some(base) = self.base else {
            self.base = Some(now);
            return;
        };
        let wall = now.wall_us.wrapping_sub(base.wall_us);
        if wall < WINDOW_US {
            return;
        }
        let mut said = [0u8; CORES];
        for (core, share) in said.iter_mut().enumerate() {
            let idle = now.idle_us[core].wrapping_sub(base.idle_us[core]);
            // the two counters are read a moment apart: an idle task that ran on past the clock's reading was idle all the window
            let idle = idle.min(wall);
            let busy = u64::from(wall - idle);
            *share = ((busy * 100 + u64::from(wall) / 2) / u64::from(wall)) as u8;
        }
        self.said = Some(said);
        self.base = Some(now);
    }

    /// The load of each core in the last window over, in per cent; None before the first window is over.
    pub fn percents(&self) -> [Option<u8>; CORES] {
        match self.said {
            Some(said) => said.map(Some),
            None => [None; CORES],
        }
    }
}
