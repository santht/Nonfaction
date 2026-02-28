use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, connect_info::ConnectInfo},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<IpAddr, RequestWindow>>>,
    max_requests: u32,
    window: Duration,
}

#[derive(Debug, Clone, Copy)]
struct RequestWindow {
    started_at: Instant,
    count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitRejection {
    pub retry_after_secs: u64,
}

impl RateLimiter {
    pub fn per_minute(max_requests: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(60),
        }
    }

    pub fn check_ip(&self, ip: IpAddr) -> Result<(), RateLimitRejection> {
        self.check_ip_at(ip, Instant::now())
    }

    fn check_ip_at(&self, ip: IpAddr, now: Instant) -> Result<(), RateLimitRejection> {
        let mut windows = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = windows.entry(ip).or_insert(RequestWindow {
            started_at: now,
            count: 0,
        });

        let elapsed = now.saturating_duration_since(entry.started_at);
        if elapsed >= self.window {
            entry.started_at = now;
            entry.count = 0;
        }

        if entry.count < self.max_requests {
            entry.count += 1;
            return Ok(());
        }

        let remaining = self.window.saturating_sub(elapsed);
        Err(RateLimitRejection {
            retry_after_secs: remaining.as_secs().max(1),
        })
    }
}

pub async fn middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(&request);
    match limiter.check_ip(ip) {
        Ok(()) => next.run(request).await,
        Err(rejection) => {
            let retry_after = HeaderValue::from_str(&rejection.retry_after_secs.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1"));
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry_after)],
                "rate limit exceeded",
            )
                .into_response()
        }
    }
}

fn client_ip(request: &Request) -> IpAddr {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .and_then(|v| v.parse::<IpAddr>().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<IpAddr>().ok())
        })
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0.ip())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ip(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, n))
    }

    #[test]
    fn allows_up_to_limit_per_window() {
        let limiter = RateLimiter::per_minute(2);
        let now = Instant::now();

        assert!(limiter.check_ip_at(test_ip(1), now).is_ok());
        assert!(limiter.check_ip_at(test_ip(1), now).is_ok());
        assert_eq!(
            limiter.check_ip_at(test_ip(1), now),
            Err(RateLimitRejection {
                retry_after_secs: 60
            })
        );
    }

    #[test]
    fn resets_window_after_a_minute() {
        let limiter = RateLimiter::per_minute(1);
        let now = Instant::now();

        assert!(limiter.check_ip_at(test_ip(1), now).is_ok());
        assert!(limiter.check_ip_at(test_ip(1), now).is_err());
        assert!(limiter
            .check_ip_at(test_ip(1), now + Duration::from_secs(61))
            .is_ok());
    }

    #[test]
    fn tracks_each_ip_independently() {
        let limiter = RateLimiter::per_minute(1);
        let now = Instant::now();

        assert!(limiter.check_ip_at(test_ip(1), now).is_ok());
        assert!(limiter.check_ip_at(test_ip(2), now).is_ok());
        assert!(limiter.check_ip_at(test_ip(1), now).is_err());
        assert!(limiter.check_ip_at(test_ip(2), now).is_err());
    }

    #[test]
    fn retry_after_reflects_remaining_seconds() {
        let limiter = RateLimiter::per_minute(1);
        let now = Instant::now();

        assert!(limiter.check_ip_at(test_ip(1), now).is_ok());
        let err = limiter
            .check_ip_at(test_ip(1), now + Duration::from_secs(15))
            .unwrap_err();
        assert_eq!(err.retry_after_secs, 45);
    }
}
