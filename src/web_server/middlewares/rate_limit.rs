//! Redis-backed rate-limit middleware.
//!
//! One time bucket per identity per second: the key embeds the current unix
//! second, so each second is a fresh window. Identity is the authenticated
//! user id when a valid bearer token is present, otherwise the client IP (so
//! unauthenticated routes like login are still limited).
//!
//!   key:   rate_limit:user:<id>:<unix_second>   (or rate_limit:ip:<addr>:<sec>)
//!   logic: INCR key; if first hit, EXPIRE; if count > limit -> 429
//!
//! Fails open: if Redis or app state is unavailable, the request proceeds.
//! The limit comes from `AppState::rate_limit_per_second` (env `RATE_LIMIT_PER_SECOND`).

use std::rc::Rc;

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpResponse,
};
use chrono::Utc;
use futures::future::{ready, LocalBoxFuture, Ready};
use serde_json::json;

use crate::applications::application_store::AppState;
use crate::utils::cache;
use crate::utils::jwt::decode_access_token;

/// Bucket lifetime. The unix-second is already in the key, so a short TTL just
/// reaps stale buckets; 2s avoids any boundary eviction races.
const BUCKET_TTL_SECS: i64 = 2;

pub struct RateLimit;

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            // Resolve limit + identity while only borrowing the request.
            let plan = req.app_data::<web::Data<AppState>>().cloned().map(|state| {
                let identity = identify(&req, &state);
                (state, identity)
            });

            if let Some((state, identity)) = plan {
                let limit = state.rate_limit_per_second;
                if limit > 0 {
                    let bucket = Utc::now().timestamp();
                    let key = format!("rate_limit:{identity}:{bucket}");
                    // Fail open on redis errors — never block traffic on a cache hiccup.
                    if let Ok(count) =
                        cache::incr_with_ttl(&state.redis_client, &key, BUCKET_TTL_SECS).await
                    {
                        if count > limit {
                            let resp = HttpResponse::TooManyRequests().json(json!({
                                "error": "rate limit exceeded",
                                "limit_per_second": limit,
                                "retry_after_seconds": 1,
                            }));
                            let (request, _) = req.into_parts();
                            return Ok(ServiceResponse::new(
                                request,
                                resp.map_into_right_body(),
                            ));
                        }
                    }
                }
            }

            let res = service.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

/// Per-user identity from a valid bearer token, else per-IP.
fn identify(req: &ServiceRequest, state: &AppState) -> String {
    if let Some(token) = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        if let Ok(claims) = decode_access_token(token, &state.jwt_secret) {
            return format!("user:{}", claims.sub);
        }
    }

    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();
    format!("ip:{ip}")
}
