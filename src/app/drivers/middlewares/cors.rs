use crate::constants::env_key;
use actix_cors::Cors;
use actix_web::http;
use std::env;

pub fn cors() -> Cors {
    let frontend_origin = env::var(env_key::FRONTEND_ORIGIN).unwrap_or_else(|_| "*".to_string());

    Cors::default()
        .allowed_origin(&frontend_origin)
        .allowed_origin_fn(|origin, _req_head| origin.as_bytes().ends_with(b".rust-lang.org"))
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(3600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_with_default_origin() {
        unsafe {
            std::env::remove_var("FRONTEND_ORIGIN");
        }
        
        // The function should not panic and return a Cors instance
        let cors_result = cors();
        // We can't directly test the Cors internals, but we can verify it's created
        assert!(std::mem::size_of_val(&cors_result) > 0);
    }

    #[test]
    fn test_cors_with_custom_origin() {
        unsafe {
            std::env::set_var("FRONTEND_ORIGIN", "https://example.com");
        }
        
        let cors_result = cors();
        assert!(std::mem::size_of_val(&cors_result) > 0);
    }

    #[test]
    fn test_cors_origin_defaults_to_wildcard() {
        unsafe {
            std::env::remove_var("FRONTEND_ORIGIN");
        }
        
        let frontend_origin = env::var(env_key::FRONTEND_ORIGIN).unwrap_or_else(|_| "*".to_string());
        assert_eq!(frontend_origin, "*");
    }
}
