//! Which paired headphones to try, and whether to try at all. No Windows
//! dependency: built and tested on every platform.
//!
//! A connection attempt to headphones that are off or with another device
//! pages them over the radio until it times out. Trying every 10 seconds all
//! day keeps the radio busy and, with multipoint headphones, could pull them
//! away from a phone (audit of Knurl 2026-11-26, point 1). So Knurl tries
//! only the headphones Windows sees connected, and the others once every
//! [`FALLBACK_EVERY`], in case Windows reports the status wrong.

use std::time::Duration;

/// How often headphones Windows does not see connected are tried anyway.
pub const FALLBACK_EVERY: Duration = Duration::from_secs(120);

/// What the connection loop does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// Nothing supported is paired.
    NonePaired,
    /// Try the paired headphones at this index.
    Try(usize),
    /// Do not try now: none is connected and the fallback is not due. The
    /// index names the headphones to show as waiting.
    Wait(usize),
}

/// Decides for `connected`, one entry per paired supported headphones:
/// `Some(true)` when Windows sees them connected, `Some(false)` when not,
/// `None` when the status could not be read (tried, as before the check).
/// `turn` rotates among several candidates; `since_fallback` is the time
/// since the last try of headphones not seen connected (`None`: never).
pub fn choose(connected: &[Option<bool>], turn: usize, since_fallback: Option<Duration>) -> Choice {
    if connected.is_empty() {
        return Choice::NonePaired;
    }
    let candidates: Vec<usize> = connected
        .iter()
        .enumerate()
        .filter(|(_, c)| **c != Some(false))
        .map(|(i, _)| i)
        .collect();
    if !candidates.is_empty() {
        return Choice::Try(candidates[turn % candidates.len()]);
    }
    let pick = turn % connected.len();
    if since_fallback.is_none_or(|d| d >= FALLBACK_EVERY) {
        Choice::Try(pick)
    } else {
        Choice::Wait(pick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOON: Option<Duration> = Some(Duration::from_secs(10));

    #[test]
    fn nothing_paired_is_said_so() {
        assert_eq!(choose(&[], 0, None), Choice::NonePaired);
    }

    #[test]
    fn connected_headphones_are_tried_at_once() {
        assert_eq!(choose(&[Some(true)], 0, SOON), Choice::Try(0));
    }

    #[test]
    fn headphones_not_connected_wait_for_the_fallback() {
        assert_eq!(choose(&[Some(false)], 0, SOON), Choice::Wait(0));
        assert_eq!(
            choose(&[Some(false)], 0, None),
            Choice::Try(0),
            "first time"
        );
        assert_eq!(
            choose(&[Some(false)], 0, Some(FALLBACK_EVERY)),
            Choice::Try(0),
            "every two minutes"
        );
    }

    #[test]
    fn an_unknown_status_is_tried_as_before() {
        assert_eq!(choose(&[None], 0, SOON), Choice::Try(0));
    }

    #[test]
    fn the_connected_pair_wins_and_several_take_turns() {
        assert_eq!(choose(&[Some(false), Some(true)], 0, SOON), Choice::Try(1));
        assert_eq!(choose(&[Some(false), Some(true)], 1, SOON), Choice::Try(1));
        assert_eq!(choose(&[Some(true), Some(true)], 1, SOON), Choice::Try(1));
        assert_eq!(
            choose(&[Some(false), Some(false)], 1, SOON),
            Choice::Wait(1)
        );
    }
}
