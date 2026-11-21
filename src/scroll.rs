//! Keeping a trackpad swipe to one axis.
//!
//! A terminal reports a trackpad as one tick per row (or column) the
//! fingers travel. Fingers swiping up or down drift sideways, which the
//! terminal reports as ticks across mixed in: Rex sent one left for every
//! two or three up. Sent to the page, they move whatever on it scrolls
//! sideways, such as a row of cards, which then snaps back.
//!
//! `AxisLock` keeps a swipe to the one direction it is mostly going, as
//! browsers do.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Ticks this far apart belong to separate swipes.
const SWIPE_GAP: Duration = Duration::from_millis(250);
/// How many of the latest ticks decide which way a swipe goes.
const RECENT: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Axis {
    Across,
    Down,
}

/// Keeps a swipe to one axis: the first tick's, until the latest ticks go
/// the other way at least twice as often.
#[derive(Default)]
pub struct AxisLock {
    recent: VecDeque<Axis>,
    across: bool,
    last: Option<Instant>,
}

impl AxisLock {
    /// The ticks (right, down) of one wheel report, with those against the
    /// swipe's direction dropped.
    pub fn filter(&mut self, now: Instant, right: i32, down: i32) -> (i32, i32) {
        if self.last.is_none_or(|last| now.saturating_duration_since(last) > SWIPE_GAP) {
            // A new swipe goes the way its first tick does. Swipes up and
            // down (Rex's, at least) start with a tick up or down, and one
            // across then needn't let drift up or down through before the
            // ticks across outnumber it.
            self.recent.clear();
            self.across = right != 0 && down == 0;
        }
        self.last = Some(now);
        let ticks = std::iter::repeat_n(Axis::Across, right.unsigned_abs() as usize)
            .chain(std::iter::repeat_n(Axis::Down, down.unsigned_abs() as usize));
        for axis in ticks {
            if self.recent.len() == RECENT {
                self.recent.pop_front();
            }
            self.recent.push_back(axis);
        }
        let across = self.recent.iter().filter(|axis| **axis == Axis::Across).count();
        let downs = self.recent.len() - across;
        // Switching takes a clear majority, so a few strays don't.
        if !self.across && across >= 2 && across >= 2 * downs {
            self.across = true;
        } else if self.across && downs >= 2 && downs >= 2 * across {
            self.across = false;
        }
        if self.across {
            (right, 0)
        } else {
            (0, down)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// About how often a terminal reports a swipe: at 60 frames a second.
    const FRAME: Duration = Duration::from_micros(16_667);

    /// Feed `ticks` (^ v < > for up, down, left, right) to a lock a frame
    /// apart, and return what it let through, the same way.
    fn locked(ticks: &str) -> String {
        let mut lock = AxisLock::default();
        let start = Instant::now();
        ticks
            .chars()
            .enumerate()
            .map(|(i, tick)| {
                let (right, down) = match tick {
                    '^' => (0, -1),
                    'v' => (0, 1),
                    '<' => (-1, 0),
                    _ => (1, 0),
                };
                match lock.filter(start + FRAME * i as u32, right, down) {
                    (0, -1) => '^',
                    (0, 1) => 'v',
                    (-1, 0) => '<',
                    (1, 0) => '>',
                    _ => '.',
                }
            })
            .collect()
    }

    #[test]
    fn a_swipe_up_that_drifts_left_stays_up_and_down() {
        // As Rex reported a swipe up on a trackpad: a tick left for every
        // two or three up.
        let swipe = "^^<^^<<^^<^^<<^^^<<^^^^<<^^^^<<<^^^<<^^^^<<<^^^<<^^^^<<<^^^<<^^^<<^^^<<<^^^<<^^<^^^<<";
        assert_eq!(locked(swipe), swipe.replace('<', "."));
        // And a stray tick across in a swipe down.
        assert_eq!(locked("v<vvvvvvv"), "v.vvvvvvv");
        assert_eq!(locked("vvvvvvvvvvvvvvvvvv>vvvvvv"), "vvvvvvvvvvvvvvvvvv.vvvvvv");
    }

    #[test]
    fn a_swipe_across_scrolls_across() {
        assert_eq!(locked(">>>>>>>>"), ">>>>>>>>");
        assert_eq!(locked("<<<<"), "<<<<");
        // Fingers swiping across drift up and down too: none of it moves
        // the page up or down, even right at the start.
        assert_eq!(locked(">v>>v>>>^>>v>>"), ">.>>.>>>.>>.>>");
        // A swipe that turns from up to across follows, once the ticks
        // across clearly outnumber the rest.
        assert_eq!(locked("^^^^<^^<>>>>>>>>>>>>>>>"), "^^^^.^^......>>>>>>>>>>");
        // And back.
        assert_eq!(locked(">>>>>>vvvvvvvvvvvv"), ">>>>>>.......vvvvv");
    }

    #[test]
    fn each_swipe_starts_afresh() {
        let mut lock = AxisLock::default();
        let start = Instant::now();
        assert_eq!(lock.filter(start, 2, 0), (2, 0));
        assert_eq!(lock.filter(start + FRAME, 1, 0), (1, 0));
        // After a pause, a swipe up and down starts with up and down.
        let later = start + SWIPE_GAP * 2;
        assert_eq!(lock.filter(later, 0, 1), (0, 1));
        assert_eq!(lock.filter(later + FRAME, 1, 0), (0, 0));
        assert_eq!(lock.filter(later + FRAME * 2, 0, 1), (0, 1));
    }
}
