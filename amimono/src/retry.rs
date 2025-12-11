use std::{fmt, ops::RangeInclusive, time::Duration};

pub trait RetryError {
    fn should_retry(&self) -> bool;
}

pub trait RetryStrategy<E>: Sync {
    fn retry(&self, completed_attempts: usize, last_error: &E) -> Option<Duration>;
}

#[derive(Clone, Debug)]
pub struct Retry {
    delay: RangeInclusive<Duration>,
    max_attempts: Option<usize>,
    factor: f64,
}

impl Retry {
    pub const fn never() -> Retry {
        Retry {
            delay: Duration::ZERO..=Duration::ZERO,
            max_attempts: Some(1),
            factor: 1.0,
        }
    }

    pub const fn immediately() -> Retry {
        Retry {
            delay: Duration::ZERO..=Duration::ZERO,
            max_attempts: None,
            factor: 1.0,
        }
    }

    pub const fn delay(dur: Duration) -> Retry {
        Retry {
            delay: dur..=dur,
            max_attempts: None,
            factor: 1.0,
        }
    }

    pub const fn delay_millis(n: u64) -> Retry {
        Self::delay(Duration::from_millis(n))
    }

    pub const fn delay_jitter(dur: RangeInclusive<Duration>) -> Retry {
        Retry {
            delay: dur,
            max_attempts: None,
            factor: 1.0,
        }
    }

    pub const fn delay_jitter_millis(n: RangeInclusive<u64>) -> Retry {
        Self::delay_jitter(Duration::from_millis(*n.start())..=Duration::from_millis(*n.end()))
    }

    pub const fn with_max_attempts(self, n: usize) -> Retry {
        Retry {
            delay: self.delay,
            max_attempts: Some(n),
            factor: self.factor,
        }
    }

    pub const fn with_backoff(self) -> Retry {
        Retry {
            delay: self.delay,
            max_attempts: self.max_attempts,
            factor: 1.5,
        }
    }
}

impl Default for Retry {
    fn default() -> Self {
        Self::never()
    }
}

impl<E: RetryError> RetryStrategy<E> for Retry {
    fn retry(&self, completed_attempts: usize, last_error: &E) -> Option<Duration> {
        let attempts_remaining = self
            .max_attempts
            .map(|x| completed_attempts < x)
            .unwrap_or(true);
        if !last_error.should_retry() || !attempts_remaining {
            return None;
        }

        let f = self
            .factor
            .powi(completed_attempts as i32 - 1)
            .clamp(1.0, 50.0);
        Some(rand::random_range(self.delay.clone()).mul_f64(f))
    }
}

pub async fn attempt<R, E, F, Fut, T>(retry: &R, op: F) -> Result<T, E>
where
    R: RetryStrategy<E>,
    E: RetryError + fmt::Display,
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    for attempt in 1.. {
        match op().await {
            Ok(x) => {
                return Ok(x);
            }
            Err(e) => match retry.retry(attempt, &e) {
                Some(dur) => {
                    log::warn!("retry after {dur:?}: {e}");
                    tokio::time::sleep(dur).await;
                }
                None => {
                    log::error!("no retries: {e}");
                    return Err(e);
                }
            },
        }
    }
    unreachable!()
}
