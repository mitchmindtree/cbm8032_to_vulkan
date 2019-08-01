use nannou::prelude::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Simple type for tracking the frames-per-second.
#[derive(Clone, Debug)]
pub struct Fps {
    window_len: usize,
    inner: RefCell<Inner>,
}

#[derive(Clone, Debug)]
struct Inner {
    window: VecDeque<Duration>,
    last: Instant,
    avg: f64,
    min: f64,
    max: f64,
}

impl Fps {
    /// The window length used by the default constructor.
    pub const DEFAULT_WINDOW_LEN: usize = 60;

    /// Create a new `Fps` with the given window length as a number of frames.
    ///
    /// The larger the window, the "smoother" the FPS.
    pub fn with_window_len(window_len: usize) -> Self {
        let window = VecDeque::with_capacity(window_len);
        let last = Instant::now();
        let (avg, min, max) = (0.0, 0.0, 0.0);
        let inner = RefCell::new(Inner {
            window,
            last,
            avg,
            min,
            max,
        });
        Fps { window_len, inner }
    }

    /// Call this to sample the rate once per frame.
    pub fn sample(&self) {
        let now = Instant::now();
        let mut inner = self.inner.borrow_mut();
        let delta = now.duration_since(inner.last);
        inner.last = now;
        inner.window.push_back(delta);
        while inner.window.len() > self.window_len {
            inner.window.pop_front();
        }
        inner.avg = inner.calc_avg();
        inner.min = inner.calc_min();
        inner.max = inner.calc_max();
    }

    /// Retrieve the last calculated frames-per-second value.
    pub fn avg(&self) -> f64 {
        self.inner.borrow().avg
    }

    /// Retrieve the last min frames per second that was reached within the window.
    pub fn min(&self) -> f64 {
        self.inner.borrow().min
    }

    /// Retrieve the last max frames per second that was reached within the window.
    pub fn max(&self) -> f64 {
        self.inner.borrow().max
    }
}

impl Inner {
    /// Calculate the frames per second from the current state of the window.
    fn calc_avg(&self) -> f64 {
        1.0 / (self.window.iter().map(|d| d.secs()).sum::<f64>() / self.window.len() as f64)
    }

    /// Find the minimum frames per second that occurs over the window.
    fn calc_min(&self) -> f64 {
        1.0 / self.window.iter().max().map(|d| d.secs()).unwrap_or(0.0)
    }

    /// Find the minimum frames per second that occurs over the window.
    fn calc_max(&self) -> f64 {
        1.0 / self.window.iter().min().map(|d| d.secs()).unwrap_or(0.0)
    }
}

impl Default for Fps {
    fn default() -> Self {
        Fps::with_window_len(Self::DEFAULT_WINDOW_LEN)
    }
}
