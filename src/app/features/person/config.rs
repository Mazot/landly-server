use super::controllers::{
    claim_confirm, claim_decline, claim_preview, create_person, fetch_person, list_people,
    vouch_person,
};
use actix_web::{web, web::ServiceConfig};

pub fn configure_services(cfg: &mut ServiceConfig) {
    cfg.service(
        web::scope("/person")
            .route("/create", web::post().to(create_person))
            .route("/list", web::get().to(list_people))
            .route("/fetch/{id}", web::get().to(fetch_person))
            .route("/vouch/{id}", web::post().to(vouch_person))
            // Claim flow is public: the token is the credential.
            .route("/claim/{token}", web::get().to(claim_preview))
            .route("/claim/{token}/confirm", web::post().to(claim_confirm))
            .route("/claim/{token}/decline", web::post().to(claim_decline)),
    );
}
