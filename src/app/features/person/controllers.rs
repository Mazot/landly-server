use super::{
    entities::ListPeopleFilters,
    requests::{
        ClaimConfirmRequest, CreatePersonRequest, ListPeopleQueryRequest, VouchPersonRequest,
    },
    usecases::{ClaimConfirmUsecaseInput, CreatePersonUsecaseInput},
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

/// The claim endpoints are public, so the auth middleware never populates
/// extensions for them. When a valid Bearer token IS supplied we decode it
/// here ourselves to link the confirming person to their account (an invalid
/// token is simply ignored — the claim token is the real credential).
fn optional_caller_user_id(req: &HttpRequest) -> Option<Uuid> {
    if let Some(user_id) = req.extensions().get::<Uuid>().copied() {
        return Some(user_id);
    }

    let header = req.headers().get("Authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;

    crate::utils::token::decode_token(token)
        .ok()
        .map(|claims| claims.sub)
}

fn parse_csv(value: &Option<String>) -> Option<Vec<String>> {
    let items: Vec<String> = value
        .as_deref()?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if items.is_empty() { None } else { Some(items) }
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/create",
    context_path = "/api",
    request_body = CreatePersonRequest,
    responses(
        (status = 200, description = "Person recommended; claim link returned for manual sending", body = super::presenters::PersonCreatedContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn create_person(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreatePersonRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .person_usecase
        .create_person(CreatePersonUsecaseInput {
            name: form.name.clone(),
            bio: form.bio.clone(),
            city: form.city.clone(),
            location_country_id: form.location_country_id,
            skills: form.skills.clone().unwrap_or_default(),
            language_ids: form.language_ids.clone().unwrap_or_default(),
            email: form.email.clone(),
            whatsapp: form.whatsapp.clone(),
            send_via: form.send_via.clone(),
            consent_given: form.consent_given,
            show_whatsapp: form.show_whatsapp.unwrap_or(false),
            show_email: form.show_email.unwrap_or(false),
            show_city: form.show_city.unwrap_or(true),
            allow_reviews: form.allow_reviews.unwrap_or(true),
            recommended_by: caller,
        })
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    get,
    path = "/person/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Public people list (confirmed/claimed only)", body = super::presenters::MultiplePeopleResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListPeopleQueryRequest),
    tag = "Person"
)]
pub async fn list_people(
    state: Data<AppState>,
    query: Query<ListPeopleQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let language_ids = parse_csv(&query.language_ids)
        .map(|ids| {
            ids.iter()
                .map(|s| Uuid::parse_str(s))
                .collect::<Result<Vec<Uuid>, _>>()
        })
        .transpose()?;

    state
        .di_container
        .person_usecase
        .list_people(ListPeopleFilters {
            city: query.city.clone(),
            skills: parse_csv(&query.skills),
            language_ids,
            limit: query.limit.unwrap_or(20).clamp(0, 100),
            offset: query.offset.unwrap_or(0).clamp(0, 500),
        })
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    get,
    path = "/person/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Person ID")
    ),
    responses(
        (status = 200, description = "Person profile (contacts gated by status + privacy)", body = super::presenters::PersonContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn fetch_person(state: Data<AppState>, id: Path<Uuid>) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .person_usecase
        .fetch_person(id.into_inner())
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/vouch/{id}",
    context_path = "/api",
    request_body = VouchPersonRequest,
    params(
        ("id" = Uuid, Path, description = "Person ID to vouch for")
    ),
    responses(
        (status = 200, description = "Vouch recorded; updated person returned", body = super::presenters::PersonContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity (not public / already vouched)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn vouch_person(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
    form: Json<VouchPersonRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .person_usecase
        .vouch_person(id.into_inner(), caller, form.note.clone())
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
// Public: the claim token itself is the credential.
#[utoipa::path(
    get,
    path = "/person/claim/{token}",
    context_path = "/api",
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Claim preview for the recommended person", body = super::presenters::ClaimPreviewContent),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Token used or expired", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_preview(
    state: Data<AppState>,
    token: Path<String>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_preview(&token)
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/claim/{token}/confirm",
    context_path = "/api",
    request_body = ClaimConfirmRequest,
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Person confirmed (claimed when authenticated)", body = super::presenters::ClaimPreviewContent),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Token used/expired or person already resolved", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_confirm(
    req: HttpRequest,
    state: Data<AppState>,
    token: Path<String>,
    form: Json<ClaimConfirmRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_confirm(
        &token,
        ClaimConfirmUsecaseInput {
            claimed_by: optional_caller_user_id(&req),
            show_whatsapp: form.show_whatsapp,
            show_email: form.show_email,
            show_city: form.show_city,
            allow_reviews: form.allow_reviews,
        },
    )
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/claim/{token}/decline",
    context_path = "/api",
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Recommendation declined"),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Token used/expired", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_decline(
    state: Data<AppState>,
    token: Path<String>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_decline(&token)
}
