//! Session authentication middleware.
//!
//! Wrapped per protected resource via `.wrap(SessionMiddleware)`. It reads the
//! `Authorization: Bearer <jwt>` header, validates it
//! (`compute::authorization::authenticate`), and on success stashes the
//! resulting `AuthUser` in the request extensions for handlers to extract.
//! On failure it short-circuits with `401` — the handler never runs.
//!
//! Handlers then take an `AuthUser` parameter (see the `FromRequest` impl at
//! the bottom), so authentication is enforced at the route boundary rather
//! than inside each handler.

use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Payload, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, FromRequest, HttpMessage, HttpRequest, ResponseError,
};
use futures::future::LocalBoxFuture;

use crate::applications::application_store::AppState;
use crate::compute::authorization::authenticate;
use crate::errors::ApiError;
use crate::models::AuthUser;

pub struct SessionMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SessionMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = SessionMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SessionMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct SessionMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SessionMiddlewareService<S>
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
            let auth_result = {
                let state = req.app_data::<web::Data<AppState>>().cloned();
                let token = req
                    .headers()
                    .get("Authorization")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(str::to_string);

                match (state, token) {
                    (Some(state), Some(token)) => authenticate(&token, &state.jwt_secret),
                    _ => Err(ApiError::Unauthorized(
                        "missing or invalid authorization header".into(),
                    )),
                }
            };

            match auth_result {
                Ok(auth_user) => {
                    req.extensions_mut().insert(auth_user);
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(err) => {
                    let resp = err.error_response();
                    let (request, _) = req.into_parts();
                    Ok(ServiceResponse::new(request, resp.map_into_right_body()))
                }
            }
        })
    }
}

/// Extractor: pulls the `AuthUser` that `SessionMiddleware` placed in the
/// request extensions. If the route wasn't wrapped with `SessionMiddleware`
/// (so no `AuthUser` is present), extraction fails with `401`.
impl FromRequest for AuthUser {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| ApiError::Unauthorized("authentication required".into()));
        ready(result)
    }
}
