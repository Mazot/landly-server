use crate::constants::env_key;
use actix_governor::{
    GovernorConfig, GovernorConfigBuilder, KeyExtractor, PeerIpKeyExtractor,
    SimpleKeyExtractionError, governor::middleware::NoOpMiddleware,
};
use actix_web::dev::ServiceRequest;
use std::net::IpAddr;

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
