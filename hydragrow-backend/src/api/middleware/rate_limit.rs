use actix_web::{
    Error,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    error::ErrorTooManyRequests,
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{debug, warn};

struct ClientData {
    count: u32,
    window_start: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, ClientData>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_duration_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_duration: Duration::from_secs(window_duration_secs),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterMiddleware {
            service,
            state: self.state.clone(),
            max_requests: self.max_requests,
            window_duration: self.window_duration,
        }))
    }
}

pub struct RateLimiterMiddleware<S> {
    service: S,
    state: Arc<Mutex<HashMap<String, ClientData>>>,
    max_requests: u32,
    window_duration: Duration,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let client_ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown_ip")
            .to_string();

        let now = Instant::now();
        let mut is_allowed = true;

        {
            let mut state = self.state.lock().unwrap();

            if state.len() > 10_000 {
                state
                    .retain(|_, data| now.duration_since(data.window_start) < self.window_duration);
            }

            let client_data = state.entry(client_ip.clone()).or_insert(ClientData {
                count: 0,
                window_start: now,
            });

            if now.duration_since(client_data.window_start) >= self.window_duration {
                client_data.count = 1;
                client_data.window_start = now;
            } else {
                client_data.count += 1;
                if client_data.count > self.max_requests {
                    is_allowed = false;
                }
            }

            debug!(
                "Rate Limit IP {}: {}/{}",
                client_ip, client_data.count, self.max_requests
            );
        } // <--- Drop lock tại đây để các request khác không bị block

        if !is_allowed {
            warn!(
                "DDoS/Spam trigger: IP {} vượt quá giới hạn {} req/phút tại {}",
                client_ip,
                self.max_requests,
                req.path()
            );

            let err = ErrorTooManyRequests(serde_json::json!({
                "error": "Too Many Requests",
                "message": format!("Bạn đã vượt quá giới hạn {} requests / {} giây. Vui lòng chờ.", self.max_requests, self.window_duration.as_secs())
            }));

            return Box::pin(async move { Err(err) });
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
