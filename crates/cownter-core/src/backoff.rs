use crate::sources::FetchError;
use std::time::Duration;

const ERROR_STEPS: [u64; 3] = [30, 60, 300];
const MAX_RATE_MULTIPLIER: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollState {
    pub consecutive_errors: u32,
    pub rate_multiplier: u32,
    pub paused: bool,
}

impl Default for PollState {
    fn default() -> Self {
        PollState { consecutive_errors: 0, rate_multiplier: 1, paused: false }
    }
}

pub enum Outcome<'a> {
    Ok,
    Err(&'a FetchError),
}

/// Delay before the next poll; `None` means polling is paused until the entry is edited.
pub fn next_delay(state: &mut PollState, interval_s: u32, outcome: Outcome) -> Option<Duration> {
    let interval = interval_s.max(1) as u64;
    match outcome {
        Outcome::Ok => {
            *state = PollState::default();
            Some(Duration::from_secs(interval))
        }
        Outcome::Err(FetchError::Auth(_)) => {
            state.paused = true;
            None
        }
        Outcome::Err(FetchError::RateLimited { retry_after_s }) => {
            state.rate_multiplier = (state.rate_multiplier * 2).min(MAX_RATE_MULTIPLIER);
            Some(Duration::from_secs(match retry_after_s {
                Some(s) => (*s).max(1),
                None => interval * state.rate_multiplier as u64,
            }))
        }
        Outcome::Err(_) => {
            let step = state.consecutive_errors as usize;
            state.consecutive_errors = state.consecutive_errors.saturating_add(1);
            let secs = match ERROR_STEPS.get(step) {
                Some(s) => *s,
                None => interval.max(300),
            };
            Some(Duration::from_secs(secs))
        }
    }
}

/// Adds 0–10 % random delay so many entries never poll in the same instant.
pub fn jitter(d: Duration, rng: &mut impl rand::Rng) -> Duration {
    let extra = d.as_secs_f64() * rng.random_range(0.0..=0.1);
    d + Duration::from_secs_f64(extra)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(d: Option<Duration>) -> u64 {
        d.unwrap().as_secs()
    }

    #[test]
    fn ok_uses_interval_and_resets() {
        let mut s = PollState::default();
        let e = FetchError::Network("x".into());
        next_delay(&mut s, 600, Outcome::Err(&e));
        next_delay(&mut s, 600, Outcome::Err(&FetchError::RateLimited { retry_after_s: None }));
        assert_eq!(secs(next_delay(&mut s, 600, Outcome::Ok)), 600);
        assert_eq!(s, PollState::default());
    }

    #[test]
    fn error_sequence() {
        let mut s = PollState::default();
        let e = FetchError::Network("x".into());
        let got: Vec<u64> = (0..5).map(|_| secs(next_delay(&mut s, 600, Outcome::Err(&e)))).collect();
        assert_eq!(got, vec![30, 60, 300, 600, 600]);
    }

    #[test]
    fn error_sequence_short_interval() {
        let mut s = PollState::default();
        let e = FetchError::Timeout;
        let got: Vec<u64> = (0..4).map(|_| secs(next_delay(&mut s, 20, Outcome::Err(&e)))).collect();
        assert_eq!(got, vec![30, 60, 300, 300]);
    }

    #[test]
    fn retry_after_wins() {
        let mut s = PollState::default();
        let e = FetchError::RateLimited { retry_after_s: Some(120) };
        assert_eq!(secs(next_delay(&mut s, 60, Outcome::Err(&e))), 120);
    }

    #[test]
    fn rate_limit_doubles() {
        let mut s = PollState::default();
        let e = FetchError::RateLimited { retry_after_s: None };
        assert_eq!(secs(next_delay(&mut s, 60, Outcome::Err(&e))), 120);
        assert_eq!(secs(next_delay(&mut s, 60, Outcome::Err(&e))), 240);
        assert_eq!(secs(next_delay(&mut s, 60, Outcome::Ok)), 60);
    }

    #[test]
    fn auth_pauses() {
        let mut s = PollState::default();
        assert_eq!(next_delay(&mut s, 60, Outcome::Err(&FetchError::Auth(401))), None);
        assert!(s.paused);
    }

    #[test]
    fn jitter_bounds() {
        let mut rng = rand::rng();
        for _ in 0..1000 {
            let d = jitter(Duration::from_secs(100), &mut rng);
            assert!(d >= Duration::from_secs(100) && d <= Duration::from_secs(110));
        }
    }
}
