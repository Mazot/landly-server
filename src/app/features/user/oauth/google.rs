use crate::app::features::user::presenters::AuthUserContent;
use crate::constants::env_key;
use crate::error::AppError;
use crate::utils::cache::{CacheService, TypedCache};
use actix_web::cookie::{Cookie, SameSite, time::Duration as CookieDuration};
use oauth2::basic::BasicTokenType;
use oauth2::url::Url;
use oauth2::{AuthUrl, TokenUrl, basic::BasicClient};
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields, EndpointNotSet,
    EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope,
    StandardRevocableToken, StandardTokenResponse, TokenResponse, reqwest,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// HttpOnly cookie that binds the OAuth `state` to the browser that started
/// the flow. Without it an attacker could start a login themselves and send
/// the victim their callback URL (login CSRF).
pub const STATE_COOKIE: &str = "landly_oauth_google_state";
const LOGIN_CODE_TTL_SECS: u64 = 60;

fn login_code_key(code: &str) -> String {
    format!("oauth:google:login-code:{}", code)
}
const STATE_COOKIE_PATH: &str = "/api/user/oauth/google";
const STATE_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct OAuthGoogle {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>,
    cache: TypedCache<Arc<dyn CacheService>>,
    /// Mark the state cookie `Secure` when the callback is served over https.
    secure_cookie: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: Option<String>,
    /// Google marks addresses it has not verified (e.g. a non-Gmail address
    /// typed at account creation); those must never be trusted as identity.
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// The callback's `state` must equal the one stored in this browser's
/// cookie. Compared in constant time.
pub fn state_matches_cookie(cookie_state: Option<&str>, state: &str) -> bool {
    let Some(cookie_state) = cookie_state else {
        return false;
    };
    if cookie_state.is_empty() || cookie_state.len() != state.len() {
        return false;
    }

    cookie_state
        .bytes()
        .zip(state.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

impl GoogleUserInfo {
    /// The email only when Google vouches for it. An unverified address is
    /// attacker-controlled and would let them sign in as whoever owns it.
    pub fn verified_email(&self) -> Result<String, AppError> {
        match (&self.email, self.email_verified) {
            (Some(email), Some(true)) => Ok(email.to_owned()),
            (None, _) => Err(AppError::Unauthorized(
                json!({ "error": "Google account has no email" }),
            )),
            _ => Err(AppError::Unauthorized(
                json!({ "error": "Google account email is not verified" }),
            )),
        }
    }
}

impl OAuthGoogle {
    /// Builds the Google OAuth client when all GOOGLE_*/OAUTH_* env vars are
    /// present; returns None otherwise so the server can start without OAuth
    /// configured (mirrors the S3/Redis optional-backend pattern).
    pub fn try_new(cache: TypedCache<Arc<dyn CacheService>>) -> Option<Self> {
        let client_id = std::env::var(env_key::GOOGLE_CLIENT_ID).ok()?;
        let client_secret = std::env::var(env_key::GOOGLE_CLIENT_SECRET).ok()?;
        let redirect = std::env::var(env_key::OAUTH_GOOGLE_REDIRECT_URL).ok()?;

        let auth_url =
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap();
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap();

        let secure_cookie = redirect.starts_with("https://");
        let redirect_url = match RedirectUrl::new(redirect) {
            Ok(url) => url,
            Err(e) => {
                log::error!("OAUTH_GOOGLE_REDIRECT_URL is not a valid URL: {e}");
                return None;
            }
        };

        let client = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_redirect_uri(redirect_url)
            .set_token_uri(token_url)
            .set_auth_uri(auth_url)
            .set_revocation_url(
                RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
                    .expect("Invalid revocation endpoint URL"),
            );

        Some(Self {
            client,
            cache,
            secure_cookie,
        })
    }

    pub fn auth_url(&self) -> Result<(Url, CsrfToken), AppError> {
        // Google supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL to which we'll redirect the user.
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            // TODO: We can change scopes and add more
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            ))
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        // Save PKCE verifier in cache
        let key = format!("oauth:google:pkce:{}", csrf_token.secret());
        let verifier = pkce_code_verifier.secret().to_string();

        self.cache.set(
            &key,
            &verifier,
            Some(std::time::Duration::from_secs(STATE_TTL_SECS)),
        )?;

        Ok((auth_url, csrf_token))
    }

    /// Cookie carrying the `state` to the callback (same-site top-level
    /// navigations only, never readable from JS).
    pub fn state_cookie(&self, state: &CsrfToken) -> Cookie<'static> {
        Cookie::build(STATE_COOKIE, state.secret().to_owned())
            .path(STATE_COOKIE_PATH)
            .http_only(true)
            .secure(self.secure_cookie)
            .same_site(SameSite::Lax)
            .max_age(CookieDuration::seconds(STATE_TTL_SECS as i64))
            .finish()
    }

    /// Expires the state cookie once the callback consumed it.
    pub fn clear_state_cookie(&self) -> Cookie<'static> {
        let mut cookie = Cookie::build(STATE_COOKIE, "")
            .path(STATE_COOKIE_PATH)
            .http_only(true)
            .secure(self.secure_cookie)
            .same_site(SameSite::Lax)
            .finish();
        cookie.make_removal();

        cookie
    }

    /// Parks the issued session under a single-use, short-lived code so the
    /// callback redirect never carries the JWT itself (URLs leak through
    /// browser history, proxy/CDN logs and the Referer header).
    pub fn issue_login_code(&self, auth: &AuthUserContent) -> Result<String, AppError> {
        let code = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());

        self.cache.set(
            &login_code_key(&code),
            auth,
            Some(std::time::Duration::from_secs(LOGIN_CODE_TTL_SECS)),
        )?;

        Ok(code)
    }

    /// Exchanges a login code for the session exactly once.
    pub fn redeem_login_code(&self, code: &str) -> Result<AuthUserContent, AppError> {
        let key = login_code_key(code);
        let auth: AuthUserContent = self.cache.get(&key)?.ok_or_else(|| {
            AppError::Unauthorized(json!({ "error": "Invalid or expired login code" }))
        })?;

        self.cache.delete(&key)?;

        Ok(auth)
    }

    // ! Example URL http://localhost:8080/api/user/oauth/google/callback?state=state&code=code&scope=email+https://www.googleapis.com/auth/userinfo.email+openid&authuser=0&prompt=consent
    pub async fn exchange_and_userinfo(
        &self,
        code: String,
        state: String,
    ) -> Result<GoogleUserInfo, AppError> {
        let key = format!("oauth:google:pkce:{}", state);
        let verifier: String = self.cache.get(&key)?.ok_or(AppError::Unauthorized(json!(
            "Invalid CSRF token".to_string()
        )))?;

        let _ = self.cache.delete(&key);

        let http_client = Self::make_http_client();

        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(verifier))
            .request_async(&http_client)
            .await
            .map_err(|_| AppError::InternalServerError)?;

        let access_token = token_result.access_token().secret();

        let user_info_response = http_client
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        let user_info = user_info_response
            .json::<GoogleUserInfo>()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        Ok(user_info)
    }

    pub async fn revoke_token(
        &self,
        token: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    ) -> Result<(), AppError> {
        let http_client = Self::make_http_client();

        let token_to_revoke: StandardRevocableToken = match token.refresh_token() {
            Some(token) => token.into(),
            None => token.access_token().into(),
        };

        self.client
            .revoke_token(token_to_revoke)
            .expect("Failed to create revocation request")
            .request_async(&http_client)
            .await
            .map_err(|_| AppError::InternalServerError)
    }

    fn make_http_client() -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to build HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;

    /// In-memory cache so the login-code round trip can be exercised.
    #[derive(Default)]
    struct MemoryCache(Mutex<HashMap<String, String>>);

    impl CacheService for MemoryCache {
        fn get_string(&self, key: &str) -> Result<Option<String>, AppError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn set_string(
            &self,
            key: &str,
            value: &str,
            _ttl: Option<Duration>,
        ) -> Result<(), AppError> {
            self.0
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }
        fn delete(&self, key: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
        fn exists(&self, key: &str) -> Result<bool, AppError> {
            Ok(self.0.lock().unwrap().contains_key(key))
        }
        fn invalidate_pattern(&self, _pattern: &str) -> Result<(), AppError> {
            Ok(())
        }
        fn mget_string(&self, keys: &[String]) -> Result<Vec<Option<String>>, AppError> {
            keys.iter().map(|k| self.get_string(k)).collect()
        }
        fn mset_string(
            &self,
            items: &[(String, String)],
            ttl: Option<Duration>,
        ) -> Result<(), AppError> {
            for (k, v) in items {
                self.set_string(k, v, ttl)?;
            }
            Ok(())
        }
    }

    fn oauth_with_memory_cache() -> OAuthGoogle {
        let client = BasicClient::new(ClientId::new("client-id".to_string()))
            .set_client_secret(ClientSecret::new("secret".to_string()))
            .set_redirect_uri(
                RedirectUrl::new("https://api.example.com/callback".to_string()).unwrap(),
            )
            .set_token_uri(
                TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap(),
            )
            .set_auth_uri(
                AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
            )
            .set_revocation_url(
                RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string()).unwrap(),
            );

        OAuthGoogle {
            client,
            cache: TypedCache::new(Arc::new(MemoryCache::default()) as Arc<dyn CacheService>),
            secure_cookie: true,
        }
    }

    #[test]
    fn test_login_code_is_single_use_and_hides_the_token() {
        let oauth = oauth_with_memory_cache();
        let auth = AuthUserContent {
            id: uuid::Uuid::new_v4(),
            username: "u".to_string(),
            email: "u@example.com".to_string(),
            token: "jwt-token".to_string(),
        };

        let code = oauth.issue_login_code(&auth).unwrap();
        assert!(!code.contains("jwt-token"));
        assert!(code.len() >= 64, "code must be unguessable");

        let redeemed = oauth.redeem_login_code(&code).unwrap();
        assert_eq!(redeemed.token, "jwt-token");

        match oauth.redeem_login_code(&code) {
            Err(AppError::Unauthorized(_)) => (),
            other => panic!("second redemption must fail, got {:?}", other.err()),
        }
    }

    #[test]
    fn test_unknown_login_code_is_rejected() {
        assert!(oauth_with_memory_cache().redeem_login_code("nope").is_err());
    }

    fn info(email: Option<&str>, email_verified: Option<bool>) -> GoogleUserInfo {
        GoogleUserInfo {
            sub: "google-sub".to_string(),
            email: email.map(str::to_string),
            email_verified,
            name: None,
            picture: None,
        }
    }

    #[test]
    fn test_state_must_match_the_browser_cookie() {
        assert!(state_matches_cookie(Some("abc123"), "abc123"));

        assert!(!state_matches_cookie(None, "abc123"));
        assert!(!state_matches_cookie(Some(""), ""));
        assert!(!state_matches_cookie(Some("abc124"), "abc123"));
        assert!(!state_matches_cookie(Some("abc"), "abc123"));
    }

    #[test]
    fn test_verified_email_accepts_verified_address() {
        let email = info(Some("a@example.com"), Some(true))
            .verified_email()
            .unwrap();

        assert_eq!(email, "a@example.com");
    }

    #[test]
    fn test_verified_email_rejects_unverified_or_missing() {
        for user_info in [
            info(Some("a@example.com"), Some(false)),
            info(Some("a@example.com"), None),
            info(None, Some(true)),
        ] {
            match user_info.verified_email() {
                Err(AppError::Unauthorized(_)) => (),
                other => panic!("expected Unauthorized, got {:?}", other),
            }
        }
    }
}
