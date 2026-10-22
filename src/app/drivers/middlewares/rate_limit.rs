use crate::constants::env_key;
use actix_governor::{
    GovernorConfig, GovernorConfigBuilder, KeyExtractor, PeerIpKeyExtractor,
    SimpleKeyExtractionError, governor::middleware::NoOpMiddleware,
};
use actix_web::{HttpMessage, dev::ServiceRequest, http::StatusCode};
use std::net::IpAddr;
use uuid::Uuid;

/// Per-client rate limiter config. Build each one ONCE at startup and share
/// it (`Governor::new(&config)`) across workers so they count together.
pub type RateLimitConfig = GovernorConfig<ClientIpKeyExtractor, NoOpMiddleware>;

/// Credential endpoints (signin, signup, OAuth): a burst of 10, then one
/// attempt every 6 s per client — enough for humans, useless for brute force
/// or mass registration.
const AUTH_BURST: u32 = 10;
const AUTH_MILLIS_PER_REQUEST: u64 = 6_000;

/// Whole API: a burst of 100, then 20 requests/s per client.
const API_BURST: u32 = 100;
const API_MILLIS_PER_REQUEST: u64 = 50;

/// Organisation submissions: each one can cost a paid Google Places lookup
/// and adds an item to the moderation queue. Per user: a burst of 5 (adding
/// a few places in one sitting), then one per 30 s. Per IP: a burst of 10,
/// then one per 10 s — accounts are free to create, so a per-user limit alone
/// could be sidestepped by registering more of them.
const SUBMIT_USER_BURST: u32 = 5;
const SUBMIT_USER_MILLIS_PER_REQUEST: u64 = 30_000;
const SUBMIT_IP_BURST: u32 = 10;
const SUBMIT_IP_MILLIS_PER_REQUEST: u64 = 10_000;

/// Per-user limiter config (keyed by the authenticated user id).
pub type UserRateLimitConfig = GovernorConfig<UserKeyExtractor, NoOpMiddleware>;

/// Every limiter that is wired into a route, built ONCE at startup and
/// cloned into each worker (clones share the counters).
#[derive(Clone)]
pub struct RateLimits {
    /// Credential routes (signin, signup, OAuth).
    pub auth: RateLimitConfig,
    /// `POST /organisation/create`, per client IP.
    pub submit_ip: RateLimitConfig,
    /// `POST /organisation/create`, per user.
    pub submit_user: UserRateLimitConfig,
}

impl RateLimits {
    pub fn new() -> Self {
        Self {
            auth: auth_rate_limit(),
            submit_ip: build(SUBMIT_IP_BURST, SUBMIT_IP_MILLIS_PER_REQUEST),
            submit_user: build_user(SUBMIT_USER_BURST, SUBMIT_USER_MILLIS_PER_REQUEST),
        }
    }

    /// Same wiring with limits no test will reach, for tests that are not
    /// about rate limiting.
    #[cfg(test)]
    pub fn relaxed() -> Self {
        Self {
            auth: build(100_000, 1),
            submit_ip: build(100_000, 1),
            submit_user: build_user(100_000, 1),
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self::new()
    }
}

pub fn auth_rate_limit() -> RateLimitConfig {
    build(AUTH_BURST, AUTH_MILLIS_PER_REQUEST)
}

pub fn api_rate_limit() -> RateLimitConfig {
    build(API_BURST, API_MILLIS_PER_REQUEST)
}

fn build(burst: u32, millis_per_request: u64) -> RateLimitConfig {
    GovernorConfigBuilder::default()
        .key_extractor(ClientIpKeyExtractor::from_env())
        .milliseconds_per_request(millis_per_request)
        .burst_size(burst)
        .finish()
        .expect("rate limit burst and period are non-zero constants")
}

fn build_user(burst: u32, millis_per_request: u64) -> UserRateLimitConfig {
    GovernorConfigBuilder::default()
        .key_extractor(UserKeyExtractor)
        .milliseconds_per_request(millis_per_request)
        .burst_size(burst)
        .finish()
        .expect("rate limit burst and period are non-zero constants")
}

/// Keys the limiter by the authenticated user id.
///
/// Only valid on routes behind the `Authentication` middleware, which has
/// already put the id into the request extensions by the time a route-level
/// limiter runs. Reaching it without one is a wiring bug: answer 401 rather
/// than letting everyone share a bucket.
#[derive(Clone, Debug)]
pub struct UserKeyExtractor;

impl KeyExtractor for UserKeyExtractor {
    type Key = Uuid;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        req.extensions().get::<Uuid>().copied().ok_or_else(|| {
            SimpleKeyExtractionError::new("Missing authenticated user")
                .set_status_code(StatusCode::UNAUTHORIZED)
        })
    }
}

/// Keys the limiter by client IP.
///
/// By default that is the TCP peer. Behind a reverse proxy every request
/// would share the proxy's IP, so `TRUST_PROXY_HEADERS=true` switches to the
/// `Forwarded` / `X-Forwarded-For` client address — enable it ONLY when the
/// proxy overwrites those headers, otherwise clients can spoof their key.
#[derive(Clone, Debug)]
pub struct ClientIpKeyExtractor {
    trust_proxy_headers: bool,
}

impl ClientIpKeyExtractor {
    pub fn from_env() -> Self {
        let trust_proxy_headers = std::env::var(env_key::TRUST_PROXY_HEADERS)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        Self {
            trust_proxy_headers,
        }
    }

    fn forwarded_ip(req: &ServiceRequest) -> Option<IpAddr> {
        let info = req.connection_info();
        let addr = info.realip_remote_addr()?;

        // "1.2.3.4", "1.2.3.4:5678", "[::1]:5678" or "::1".
        addr.parse::<IpAddr>().ok().or_else(|| {
            addr.parse::<std::net::SocketAddr>()
                .ok()
                .map(|socket| socket.ip())
        })
    }
}

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        if self.trust_proxy_headers
            && let Some(ip) = Self::forwarded_ip(req)
        {
            return Ok(normalize(ip));
        }

        PeerIpKeyExtractor.extract(req)
    }
}

/// Same IPv6 handling as `PeerIpKeyExtractor`: clients usually own a /56,
/// so limit per prefix instead of per address.
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => {
            let mut octets = ipv6.octets();
            octets[7..16].fill(0);
            IpAddr::V6(octets.into())
        }
        v4 => v4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn request(peer: &str, forwarded_for: Option<&str>) -> ServiceRequest {
        let mut req = TestRequest::default().peer_addr(peer.parse().unwrap());
        if let Some(value) = forwarded_for {
            req = req.insert_header(("X-Forwarded-For", value));
        }

        req.to_srv_request()
    }

    #[test]
    fn test_uses_peer_ip_unless_proxy_headers_are_trusted() {
        let extractor = ClientIpKeyExtractor {
            trust_proxy_headers: false,
        };
        let req = request("10.0.0.1:4000", Some("203.0.113.7"));

        assert_eq!(
            extractor.extract(&req).unwrap(),
            "10.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_uses_forwarded_client_ip_behind_a_trusted_proxy() {
        let extractor = ClientIpKeyExtractor {
            trust_proxy_headers: true,
        };
        let req = request("10.0.0.1:4000", Some("203.0.113.7"));

        assert_eq!(
            extractor.extract(&req).unwrap(),
            "203.0.113.7".parse::<IpAddr>().unwrap()
        );

        // No header: fall back to the peer.
        let req = request("10.0.0.1:4000", None);
        assert_eq!(
            extractor.extract(&req).unwrap(),
            "10.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_ipv6_is_limited_per_prefix() {
        assert_eq!(
            normalize("2001:db8:1:2:3:4:5:6".parse().unwrap()),
            "2001:db8:1::".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_user_key_is_the_authenticated_user() {
        let user = Uuid::new_v4();
        let req = TestRequest::default().to_srv_request();
        req.extensions_mut().insert(user);
        assert_eq!(UserKeyExtractor.extract(&req).unwrap(), user);

        // Never share one bucket between users if the id is missing.
        let anonymous = TestRequest::default().to_srv_request();
        let err = UserKeyExtractor.extract(&anonymous).unwrap_err();
        assert_eq!(
            actix_web::ResponseError::status_code(&err),
            StatusCode::UNAUTHORIZED
        );
    }

    /// The per-user limiter counts each user separately.
    #[actix_web::test]
    async fn test_user_limit_is_per_user() {
        use actix_governor::Governor;
        use actix_web::{App, HttpMessage, HttpResponse, test, web};

        let config = RateLimits::new().submit_user;
        let app = test::init_service(
            App::new()
                .wrap_fn(|req, srv| {
                    use actix_web::dev::Service;
                    // Stand-in for the Authentication middleware: the user id
                    // comes from a header here.
                    let id = req
                        .headers()
                        .get("x-user")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<Uuid>().ok());
                    if let Some(id) = id {
                        req.extensions_mut().insert(id);
                    }
                    srv.call(req)
                })
                .route(
                    "/submit",
                    web::post()
                        .to(HttpResponse::Ok)
                        .wrap(Governor::new(&config)),
                ),
        )
        .await;
        let submit = |user: Uuid| {
            test::TestRequest::post()
                .uri("/submit")
                .insert_header(("x-user", user.to_string()))
                .peer_addr("198.51.100.9:1000".parse().unwrap())
                .to_request()
        };
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());

        for _ in 0..SUBMIT_USER_BURST {
            assert_eq!(
                test::call_service(&app, submit(a)).await.status(),
                StatusCode::OK
            );
        }
        assert_eq!(
            test::call_service(&app, submit(a)).await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            test::call_service(&app, submit(b)).await.status(),
            StatusCode::OK,
            "same IP, other user: own bucket"
        );
    }

    /// The auth limiter lets a burst through, then answers 429 — and keeps
    /// counting per client, not globally.
    #[actix_web::test]
    async fn test_auth_limit_blocks_brute_force_per_client() {
        use actix_governor::Governor;
        use actix_web::{App, HttpResponse, http::StatusCode, test, web};

        let config = auth_rate_limit();
        let app = test::init_service(
            App::new().route(
                "/signin",
                web::post()
                    .to(HttpResponse::Ok)
                    .wrap(Governor::new(&config)),
            ),
        )
        .await;

        let attempt = |peer: &'static str| {
            test::TestRequest::post()
                .uri("/signin")
                .peer_addr(peer.parse().unwrap())
                .to_request()
        };

        for _ in 0..AUTH_BURST {
            let res = test::call_service(&app, attempt("198.51.100.1:1000")).await;
            assert_eq!(res.status(), StatusCode::OK);
        }

        let res = test::call_service(&app, attempt("198.51.100.1:1000")).await;
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);

        let res = test::call_service(&app, attempt("198.51.100.2:1000")).await;
        assert_eq!(res.status(), StatusCode::OK, "other clients are unaffected");

        let _ = api_rate_limit();
    }
}
