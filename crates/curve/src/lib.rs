//! The last N readings of one value, and the mean of a run of them. No hardware, no sensor and no page: the
//! modules of the device put readings in, and whoever draws the curve takes them out.
//!
//! A ring rather than a list, because the board has about 127 KB free and a curve that grows is a board that
//! restarts three hours later. What is lost when it fills is the PAST, never the present.

pub mod degrees;

/// The last N readings, oldest first when they are read out.
///
/// N is a type parameter and not a field: the array is then part of the struct and the whole cost is known at
/// build time, which is what lets the work state the memory as a number instead of an estimate.
///
/// T is what one reading is, `u16` unless said otherwise: a sensor's whole units or tenths. A value that goes below
/// nought, lives in fractions or has seconds with nothing to say - the regulator's halves of a move - is kept as
/// `f32`, NaN where there was nothing, at twice the bytes.
pub struct Ring<const N: usize, T = u16> {
    /// Where the next reading goes.
    next: usize,
    /// How many of the N places hold a reading. Stops growing at N and is what tells a young ring from a full
    /// one - without it the zeroes of an unfilled array would be drawn as readings of zero.
    filled: usize,
    points: [T; N],
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ring<N> {
    pub const fn new() -> Self {
        Self::filled_with(0)
    }
}

impl<const N: usize, T: Copy> Ring<N, T> {
    /// An empty ring whose unfilled places hold FILL - never read out, since `filled` says how many are readings.
    pub const fn filled_with(fill: T) -> Self {
        Self { next: 0, filled: 0, points: [fill; N] }
    }

    /// An empty ring like [`Ring::filled_with`], built on the heap and NEVER ON THE STACK of the task that builds it.
    ///
    /// `Box::new(Ring::filled_with(..))` puts the whole array on the stack first and copies it over, even optimised:
    /// the regulator's six rings of 300, 5.4 KB, overflowed the 8 KB stack of `holding` that way. Here
    /// the heap is taken first and the places are written into it one by one.
    pub fn boxed(fill: T) -> Box<Self> {
        let mut ring = Box::<Self>::new_uninit();
        let at = ring.as_mut_ptr();
        // SAFETY: `at` points at memory for one Ring, allocated and not yet read; every field is written below
        // through a raw pointer before `assume_init`, and T: Copy has no drop to run on what it overwrites.
        unsafe {
            core::ptr::addr_of_mut!((*at).next).write(0);
            core::ptr::addr_of_mut!((*at).filled).write(0);
            let points = core::ptr::addr_of_mut!((*at).points) as *mut T;
            for i in 0..N {
                points.add(i).write(fill);
            }
            ring.assume_init()
        }
    }

    /// How many readings it holds - up to N.
    pub fn len(&self) -> usize {
        self.filled
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Puts a reading in, dropping the oldest when the ring is full.
    pub fn push(&mut self, value: T) {
        if N == 0 {
            return;
        }
        self.points[self.next] = value;
        self.next = (self.next + 1) % N;
        self.filled = (self.filled + 1).min(N);
    }

    /// The readings it holds, OLDEST FIRST - the order a curve is drawn in, so nobody has to unwrap the ring
    /// at the far end where the mistake would look like a jump in the data.
    pub fn points(&self) -> Vec<T> {
        let mut out = Vec::with_capacity(self.filled);
        let start = match self.filled < N {
            true => 0,
            false => self.next,
        };
        for i in 0..self.filled {
            out.push(self.points[(start + i) % N]);
        }
        out
    }
}

/// The mean of a run of N readings: the slow ring under a fast sensor.
///
/// A sensor read ten times a second fills a second's worth of the slow curve with ten readings, and one of
/// them chosen at random would be the noise rather than the value. The sum is taken in u32 - a run of u16
/// overflows a u16 after three.
pub struct Mean<const N: usize> {
    sum: u32,
    count: usize,
}

impl<const N: usize> Default for Mean<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Mean<N> {
    pub const fn new() -> Self {
        Self { sum: 0, count: 0 }
    }

    /// Adds a reading; answers the mean when the run is full, and starts the next run.
    pub fn add(&mut self, value: u16) -> Option<u16> {
        self.sum += value as u32;
        self.count += 1;
        if self.count < N.max(1) {
            return None;
        }
        // rounded to the nearest, like the window of samples: cutting leans every point of the curve one way
        let mean = ((self.sum + self.count as u32 / 2) / self.count as u32) as u16;
        self.sum = 0;
        self.count = 0;
        Some(mean)
    }
}

/// The two curves of ONE fast sensor: every reading on the fast ring, the mean of each run of RUN readings on
/// the slow one.
///
/// Two rings rather than one because the questions are different. The fast curve answers "what does this value
/// do between one second and the next" - pulsations, the kick of a regulator, the flicker of a fire. The slow
/// one answers "where has it been for the last quarter of an hour" - the drift of the gases after a portion of
/// fuel, which no fast ring could hold without eating the board's memory.
pub struct Pair<const FAST: usize, const SLOW: usize, const RUN: usize> {
    pub fast: Ring<FAST>,
    pub slow: Ring<SLOW>,
    run: Mean<RUN>,
}

impl<const FAST: usize, const SLOW: usize, const RUN: usize> Default for Pair<FAST, SLOW, RUN> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const FAST: usize, const SLOW: usize, const RUN: usize> Pair<FAST, SLOW, RUN> {
    pub const fn new() -> Self {
        Self { fast: Ring::new(), slow: Ring::new(), run: Mean::new() }
    }

    /// One reading: onto the fast curve always, onto the slow one when its run is full.
    pub fn push(&mut self, value: u16) {
        self.fast.push(value);
        if let Some(mean) = self.run.add(value) {
            self.slow.push(mean);
        }
    }
}
