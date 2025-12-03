use crate::constants::env_key;
use crate::error::AppError;
use crate::utils::cache::{CacheService, TypedCache};
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

#[derive(Clone)]
pub struct OAuthGoogle {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>,
    cache: TypedCache<Arc<dyn CacheService>>,
}

#[derive(Deserialize, ToSchema)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

impl OAuthGoogle {
    pub fn new(cache: TypedCache<Arc<dyn CacheService>>) -> Self {
        let client_id = std::env::var(env_key::GOOGLE_CLIENT_ID).expect("GOOGLE_CLIENT_ID missing");
        let client_secret =
            std::env::var(env_key::GOOGLE_CLIENT_SECRET).expect("GOOGLE_CLIENT_SECRET missing");
        let redirect = std::env::var(env_key::OAUTH_GOOGLE_REDIRECT_URL)
            .expect("OAUTH_GOOGLE_REDIRECT_URL missing");

        let auth_url =
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap();
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap();

        let client = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_redirect_uri(RedirectUrl::new(redirect).expect("Invalid redirect URL"))
            .set_token_uri(token_url)
            .set_auth_uri(auth_url)
            .set_revocation_url(
                RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
                    .expect("Invalid revocation endpoint URL"),
            );

        Self { client, cache }
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

        self.cache
            .set(&key, &verifier, Some(std::time::Duration::from_secs(300)))?;

        Ok((auth_url, csrf_token))
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
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to build HTTP client");

        http_client
    }
}
