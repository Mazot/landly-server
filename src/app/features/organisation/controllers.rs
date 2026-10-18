use super::{
    requests::{
        CheckinRequest, CreateOrganisationRequest, OrganisationsListQueryRequest,
        SearchOrganisationsQueryRequest, UpdateOrganisationRequest,
    },
    usecases::{
        CreateOrganisationUsecaseInput, FetchOrganisationsUsecaseInput,
        SearchOrganisationsUsecaseInput, UpdateOrganisationUsecaseInput,
    },
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use bigdecimal::BigDecimal;
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

const LATITUDE_RANGE: std::ops::RangeInclusive<f64> = -90.0..=90.0;
const LONGITUDE_RANGE: std::ops::RangeInclusive<f64> = -180.0..=180.0;

/// Converts an optional f64 coordinate into a BigDecimal, rejecting
/// non-finite and out-of-range values with a 422 instead of panicking
/// (or silently storing garbage like lat=999).
fn parse_coordinate(
    value: Option<f64>,
    field: &str,
    range: std::ops::RangeInclusive<f64>,
) -> Result<Option<BigDecimal>, AppError> {
    value
        .map(|v| {
            if !v.is_finite() || !range.contains(&v) {
                return Err(AppError::UnprocessableEntity(json!({
                    "error": format!(
                        "Invalid {} value: must be within [{}, {}]",
                        field,
                        range.start(),
                        range.end()
                    )
                })));
            }

            BigDecimal::try_from(v).map_err(|_| {
                AppError::UnprocessableEntity(json!({
                    "error": format!("Invalid {} value", field)
                }))
            })
        })
        .transpose()
}

fn parse_latitude(value: Option<f64>) -> Result<Option<BigDecimal>, AppError> {
    parse_coordinate(value, "latitude", LATITUDE_RANGE)
}

fn parse_longitude(value: Option<f64>) -> Result<Option<BigDecimal>, AppError> {
    parse_coordinate(value, "longitude", LONGITUDE_RANGE)
}

/// Validates a search origin/bbox coordinate pair without converting it.
fn ensure_coordinate_ranges(pairs: &[(&str, f64)]) -> Result<(), AppError> {
    for (field, value) in pairs {
        let range = if field.contains("lat") {
            LATITUDE_RANGE
        } else {
            LONGITUDE_RANGE
        };

        if !value.is_finite() || !range.contains(value) {
            return Err(AppError::UnprocessableEntity(json!({
                "error": format!(
                    "Invalid {} value: must be within [{}, {}]",
                    field,
                    range.start(),
                    range.end()
                )
            })));
        }
    }

    Ok(())
}

/// Splits a comma-separated query value into a list, dropping empty chunks.
fn parse_csv(value: &Option<String>) -> Option<Vec<String>> {
    let items: Vec<String> = value
        .as_deref()?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if items.is_empty() { None } else { Some(items) }
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// results are now filtered to status='live'.
#[utoipa::path(
    get,
    path = "/organisation/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Organisations list response", body = super::presenters::MultipleOrganisationsResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(OrganisationsListQueryRequest),
    tag = "Organisation"
)]
pub async fn list_organisations(
    state: Data<AppState>,
    query: Query<OrganisationsListQueryRequest>,
) -> Result<HttpResponse, AppError> {
    // Clamp to sane bounds: negative values reach Postgres as
    // `OFFSET -n` / `LIMIT -n` and blow up with a 500.
    let offset = query.offset.unwrap_or(0).clamp(0, 150);
    let limit = query.limit.unwrap_or(20).clamp(0, 100);

    state
        .di_container
        .organisation_usecase
        .fetch_organisations(FetchOrganisationsUsecaseInput {
            name: query.name.clone(),
            tel: query.tel.clone(),
            email: query.email.clone(),
            address: query.address.clone(),
            location_country_id: query.location_country_id,
            organisation_type_id: query.organisation_type_id,
            founder_country_id: query.founder_country_id,
            limit,
            offset,
        })
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// response carries the v2 fields incl. computed openNow.
#[utoipa::path(
    get,
    path = "/organisation/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID to fetch")
    ),
    responses(
        (status = 200, description = "Organisation fetched successfully", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn fetch_organisation(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .fetch_organisation(id.into_inner())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// ownership/RBAC check, v2 fields, panic-free coordinate parsing.
#[utoipa::path(
    put,
    path = "/organisation/update/{id}",
    context_path = "/api",
    request_body = UpdateOrganisationRequest,
    params(
        ("id" = Uuid, Path, description = "Organisation ID to update")
    ),
    responses(
        (status = 200, description = "Organisation updated; a creator edit of a live organisation sends it back to `pending`", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn update_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    form: Json<UpdateOrganisationRequest>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.organisation_usecase.update_organisation(
        id.into_inner(),
        caller,
        UpdateOrganisationUsecaseInput {
            name: form.name.clone(),
            tel: form.tel.clone(),
            email: form.email.clone(),
            address: form.address.clone(),
            description: form.description.clone(),
            location_country_id: form.location_country_id,
            organisation_type_id: form.organisation_type_id,
            founder_country_id: form.founder_country_id,
            latitude: parse_latitude(form.latitude)?,
            longitude: parse_longitude(form.longitude)?,
            city: form.city.clone(),
            website: form.website.clone(),
            telegram: form.telegram.clone(),
            whatsapp: form.whatsapp.clone(),
            services: form.services.clone(),
            languages: form.languages.clone(),
            opening_hours: form.opening_hours.clone(),
            timezone: form.timezone.clone(),
            cost: form.cost.clone(),
        },
    )
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// ownership/RBAC check.
#[utoipa::path(
    delete,
    path = "/organisation/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID to delete")
    ),
    responses(
        (status = 200, description = "Organisation deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn delete_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .organisation_usecase
        .delete_organisation(id.into_inner(), caller)
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// created_by from JWT, v2 fields, submissions start as status='pending'.
#[utoipa::path(
    post,
    path = "/organisation/create",
    context_path = "/api",
    request_body = CreateOrganisationRequest,
    responses(
        (status = 200, description = "Organisation created successfully", body = super::presenters::OrganisationContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn create_organisation(
    state: Data<AppState>,
    req: HttpRequest,
    form: Json<CreateOrganisationRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .organisation_usecase
        .create_organisation(CreateOrganisationUsecaseInput {
            name: form.name.clone(),
            tel: form.tel.clone(),
            email: form.email.clone(),
            address: form.address.clone(),
            description: form.description.clone(),
            location_country_id: form.location_country_id,
            organisation_type_id: form.organisation_type_id,
            founder_country_id: form.founder_country_id,
            latitude: parse_latitude(form.latitude)?,
            longitude: parse_longitude(form.longitude)?,
            created_by: caller,
            added_by: form.added_by.clone(),
            city: form.city.clone(),
            website: form.website.clone(),
            telegram: form.telegram.clone(),
            whatsapp: form.whatsapp.clone(),
            services: form.services.clone().unwrap_or_default(),
            languages: form.languages.clone().unwrap_or_default(),
            opening_hours: form.opening_hours.clone(),
            timezone: form.timezone.clone(),
            cost: form.cost.clone(),
            google_place_id: form.google_place_id.clone(),
        })
        .await
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/organisation/search",
    context_path = "/api",
    responses(
        (status = 200, description = "Geo search results", body = super::presenters::MultipleOrganisationsResponse),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(SearchOrganisationsQueryRequest),
    tag = "Organisation"
)]
pub async fn search_organisations(
    state: Data<AppState>,
    query: Query<SearchOrganisationsQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let bbox = match (query.min_lat, query.min_lng, query.max_lat, query.max_lng) {
        (Some(min_lat), Some(min_lng), Some(max_lat), Some(max_lng)) => {
            Some((min_lat, min_lng, max_lat, max_lng))
        }
        (None, None, None, None) => None,
        _ => {
            return Err(AppError::UnprocessableEntity(json!({
                "error": "bbox requires all of min_lat, min_lng, max_lat, max_lng"
            })));
        }
    };

    let origin = match (query.lat, query.lng) {
        (Some(lat), Some(lng)) => Some((lat, lng)),
        (None, None) => None,
        _ => {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "origin requires both lat and lng" }),
            ));
        }
    };

    if bbox.is_none() && origin.is_none() {
        return Err(AppError::UnprocessableEntity(json!({
            "error": "Provide a bbox (min_lat..max_lng) or an origin (lat, lng)"
        })));
    }

    if let Some((min_lat, min_lng, max_lat, max_lng)) = bbox {
        ensure_coordinate_ranges(&[
            ("min_lat", min_lat),
            ("min_lng", min_lng),
            ("max_lat", max_lat),
            ("max_lng", max_lng),
        ])?;
    }
    if let Some((lat, lng)) = origin {
        ensure_coordinate_ranges(&[("lat", lat), ("lng", lng)])?;
    }

    state
        .di_container
        .organisation_usecase
        .search_organisations(SearchOrganisationsUsecaseInput {
            bbox,
            origin,
            radius_km: query.radius_km,
            type_slugs: parse_csv(&query.types),
            open_now: query.open_now.unwrap_or(false),
            languages: parse_csv(&query.languages),
            verified_only: query.verified.unwrap_or(false),
            min_rating: query.min_rating,
            added_by: query.added_by.clone(),
            cost: query.cost.clone(),
            sort: query.sort.clone(),
            limit: query.limit.unwrap_or(50).clamp(0, 200),
            offset: query.offset.unwrap_or(0).max(0),
        })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    post,
    path = "/organisation/visit/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Organisation ID that was visited")
    ),
    responses(
        (status = 200, description = "Visit counted", body = super::presenters::OrganisationVisitsContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn visit_organisation(
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .visit_organisation(id.into_inner())
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    post,
    path = "/organisation/checkin/{id}",
    context_path = "/api",
    request_body = CheckinRequest,
    params(
        ("id" = Uuid, Path, description = "Organisation ID being checked into")
    ),
    responses(
        (status = 200, description = "Check-in recorded; detail payload with fresh community signals", body = super::presenters::OrganisationContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 422, description = "Organisation is not live", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Organisation"
)]
pub async fn checkin_organisation(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
    form: Json<CheckinRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .organisation_usecase
        .checkin_organisation(
            id.into_inner(),
            caller,
            form.still_active.unwrap_or(true),
            form.tip.clone(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coordinate_valid() {
        assert!(parse_latitude(Some(52.52)).unwrap().is_some());
        assert!(parse_longitude(Some(-179.9)).unwrap().is_some());
        assert!(parse_latitude(Some(-90.0)).unwrap().is_some());
        assert!(parse_longitude(Some(180.0)).unwrap().is_some());
    }

    #[test]
    fn test_parse_coordinate_none() {
        assert!(parse_latitude(None).unwrap().is_none());
        assert!(parse_longitude(None).unwrap().is_none());
    }

    #[test]
    fn test_parse_coordinate_rejects_non_finite() {
        // Previously these panicked via .expect(); now they must be a 422.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            match parse_latitude(Some(bad)) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
            }
        }
    }

    #[test]
    fn test_parse_coordinate_rejects_out_of_range() {
        assert!(parse_latitude(Some(90.01)).is_err());
        assert!(parse_latitude(Some(-999.0)).is_err());
        assert!(parse_longitude(Some(180.5)).is_err());
        assert!(parse_longitude(Some(-181.0)).is_err());
    }

    #[test]
    fn test_ensure_coordinate_ranges() {
        assert!(ensure_coordinate_ranges(&[("lat", 52.5), ("lng", 13.4)]).is_ok());
        assert!(ensure_coordinate_ranges(&[("min_lat", 999.0)]).is_err());
        assert!(ensure_coordinate_ranges(&[("max_lng", -200.0)]).is_err());
    }

    #[test]
    fn test_parse_csv_basic() {
        let input = Some("embassy,community".to_string());
        assert_eq!(
            parse_csv(&input),
            Some(vec!["embassy".to_string(), "community".to_string()])
        );
    }

    #[test]
    fn test_parse_csv_trims_and_drops_empty_chunks() {
        let input = Some(" embassy , ,community, ".to_string());
        assert_eq!(
            parse_csv(&input),
            Some(vec!["embassy".to_string(), "community".to_string()])
        );
    }

    #[test]
    fn test_parse_csv_empty_inputs() {
        assert_eq!(parse_csv(&None), None);
        assert_eq!(parse_csv(&Some("".to_string())), None);
        assert_eq!(parse_csv(&Some(", ,".to_string())), None);
    }
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::data::schema::{moderation_events, organisations};
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use diesel::prelude::*;
    use serde_json::{Value, json};
    use serial_test::serial;
    use uuid::Uuid;

    fn always_open() -> Value {
        // Closing time is exclusive, so a full day ends at 24:00.
        let day = json!([["00:00", "24:00"]]);
        json!({ "mon": day, "tue": day, "wed": day, "thu": day, "fri": day, "sat": day, "sun": day })
    }

    fn ids(body: &Value) -> Vec<String> {
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_string())
            .collect()
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_create_submits_a_pending_org_with_moderation_flags() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let country = ts::country(&pool);
        let (lat, lng) = ts::random_point();
        let name = format!("Kiezladen {}", ts::unique());

        let (status, org) = call(&app, Method::POST, "/api/organisation/create", Some(&me.token), Some(json!({
            "name": name, "tel": "+49 30 1234567", "latitude": lat, "longitude": lng,
            "location_country_id": country.id, "services": ["Help"], "languages": ["Russian"],
            "opening_hours": always_open(), "timezone": "UTC", "cost": "free", "added_by": "community",
        }))).await;
        assert_eq!(status, StatusCode::OK, "{org}");
        assert_eq!(org["status"], "pending");
        assert_eq!(org["createdBy"], json!(me.id));
        assert_eq!(org["openNow"], true);

        let id: Uuid = org["id"].as_str().unwrap().parse().unwrap();
        let conn = &mut pool.get().unwrap();
        let flags: Option<Value> = moderation_events::table
            .filter(moderation_events::target_id.eq(id))
            .filter(moderation_events::action.eq("submitted"))
            .select(moderation_events::flags)
            .first(conn)
            .unwrap();
        let flags = flags.unwrap();
        assert_eq!(flags["duplicateNearby"], false);
        assert_eq!(flags["phoneLooksValid"], true);

        // A second submission with the same name at the same spot is flagged.
        let (_, dup) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": name, "latitude": lat, "longitude": lng })),
        )
        .await;
        let dup_id: Uuid = dup["id"].as_str().unwrap().parse().unwrap();
        let flags: Option<Value> = moderation_events::table
            .filter(moderation_events::target_id.eq(dup_id))
            .select(moderation_events::flags)
            .first(conn)
            .unwrap();
        assert_eq!(flags.unwrap()["duplicateNearby"], true);
    }

    /// `longitude` used to be DECIMAL(10, 8) (max ±99.99999999): every place
    /// east of 100°E or west of 100°W — Tokyo, Bangkok, the Americas — failed
    /// with a 500 "numeric field overflow".
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_create_accepts_the_full_longitude_range() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");

        for (city, lat, lng) in [
            ("Tokyo", 35.6764, 139.6500),
            ("Los Angeles", 34.0549, -118.2426),
            ("Antimeridian", -16.5, 179.99999999),
            ("Antimeridian west", 64.8, -179.99999999),
        ] {
            let (status, org) = call(&app, Method::POST, "/api/organisation/create", Some(&me.token),
                Some(json!({ "name": format!("{city} {}", ts::unique()), "latitude": lat, "longitude": lng }))).await;
            assert_eq!(status, StatusCode::OK, "{city}: {org}");
            assert!(
                (org["longitude"].as_f64().unwrap() - lng).abs() < 1e-6,
                "{city}: stored {}",
                org["longitude"]
            );
        }
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_create_validates_input() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");

        for body in [
            json!({ "name": "x", "latitude": 91.0, "longitude": 0.0 }),
            json!({ "name": "x", "latitude": 0.0, "longitude": 181.0 }),
            json!({ "name": "x", "cost": "cheap" }),
            json!({ "name": "x", "added_by": "robot" }),
        ] {
            let (status, resp) = call(
                &app,
                Method::POST,
                "/api/organisation/create",
                Some(&me.token),
                Some(body.clone()),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body} -> {resp}");
        }
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            None,
            Some(json!({ "name": "x" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_update_and_delete_respect_ownership_and_remoderation() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (creator, stranger, moderator) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
        );
        let org = ts::org(
            &pool,
            OrgSpec {
                created_by: Some(creator.id),
                ..Default::default()
            },
        );
        let path = format!("/api/organisation/update/{}", org.id);

        let (status, _) = call(
            &app,
            Method::PUT,
            &path,
            Some(&stranger.token),
            Some(json!({ "name": "hijack" })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&moderator.token),
            Some(json!({ "description": "fixed typo" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["status"], "live", "moderator edits keep the org live");

        let (status, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&creator.token),
            Some(json!({ "website": "https://example.test" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["status"], "pending",
            "creator edits of a live org go back to moderation"
        );
        let conn = &mut pool.get().unwrap();
        let flags: Option<Value> = moderation_events::table
            .filter(moderation_events::target_id.eq(org.id))
            .select(moderation_events::flags)
            .first(conn)
            .unwrap();
        assert_eq!(flags.unwrap()["editedAfterApproval"], true);

        let delete = format!("/api/organisation/delete/{}", org.id);
        let (status, _) = call(&app, Method::DELETE, &delete, Some(&stranger.token), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = call(&app, Method::DELETE, &delete, Some(&creator.token), None).await;
        assert_eq!(status, StatusCode::OK);
        let left: i64 = organisations::table
            .filter(organisations::id.eq(org.id))
            .count()
            .get_result(conn)
            .unwrap();
        assert_eq!(left, 0);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_list_and_fetch() {
        let pool = require_db!();
        let app = test_app!(pool);
        let country = ts::country(&pool);
        let suffix = ts::unique();
        let name = format!("Listable-{suffix}");
        let live = ts::org(
            &pool,
            OrgSpec {
                country: Some(country.id),
                name: Some(name.clone()),
                ..Default::default()
            },
        );
        let pending = ts::org(
            &pool,
            OrgSpec {
                country: Some(country.id),
                status: Some("pending"),
                ..Default::default()
            },
        );

        let (status, list) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/list?location_country_id={}", country.id),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            ids(&list),
            vec![live.id.to_string()],
            "only live orgs are listed"
        );

        let (_, by_name) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/list?name={}", suffix.to_uppercase()),
            None,
            None,
        )
        .await;
        assert!(
            ids(&by_name).contains(&live.id.to_string()),
            "name filter is a case-insensitive substring"
        );

        let (status, detail) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/fetch/{}", live.id),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(detail["community"]["peopleCame"], 0);
        assert_eq!(detail["openNow"], Value::Null, "no hours → unknown");

        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/fetch/{}", Uuid::new_v4()),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // Documents current behaviour: fetch-by-id does not hide pending orgs.
        let (status, body) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/fetch/{}", pending.id),
            None,
            None,
        )
        .await;
        assert_eq!(
            (status, body["status"].as_str()),
            (StatusCode::OK, Some("pending"))
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_search_filters_and_sorting() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let kind = ts::org_type(&pool);
        let (lat, lng) = ts::random_point();
        let near = ts::org(
            &pool,
            OrgSpec {
                lat_lng: Some((lat, lng)),
                org_type: Some(kind.id),
                ..Default::default()
            },
        );
        let far = ts::org(
            &pool,
            OrgSpec {
                lat_lng: Some((lat + 0.05, lng)),
                ..Default::default()
            },
        ); // ~5.5 km north
        ts::org(
            &pool,
            OrgSpec {
                lat_lng: Some((lat, lng)),
                status: Some("pending"),
                ..Default::default()
            },
        );
        // Richer attributes go through the API so every column is set.
        let (_, rich) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({
                "name": format!("Rich {}", ts::unique()), "latitude": lat + 0.01, "longitude": lng,
                "languages": ["Ukrainian"], "opening_hours": always_open(), "timezone": "UTC",
                "cost": "paid", "added_by": "official",
            })),
        )
        .await;
        let rich_id: Uuid = rich["id"].as_str().unwrap().parse().unwrap();
        diesel::update(organisations::table.find(rich_id))
            .set((
                organisations::status.eq("live"),
                organisations::verified.eq(true),
                organisations::rating_avg.eq(Some(4.8)),
            ))
            .execute(&mut pool.get().unwrap())
            .unwrap();

        let base = format!("/api/organisation/search?lat={lat}&lng={lng}&radius_km=10");
        let (status, all) = call(&app, Method::GET, &base, None, None).await;
        assert_eq!(status, StatusCode::OK, "{all}");
        assert_eq!(
            ids(&all),
            vec![near.id.to_string(), rich_id.to_string(), far.id.to_string()],
            "nearest first, pending hidden"
        );
        assert!(all["items"][0]["distanceKm"].as_f64().unwrap() < 0.01);

        let (_, small) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/search?lat={lat}&lng={lng}&radius_km=2"),
            None,
            None,
        )
        .await;
        assert!(
            !ids(&small).contains(&far.id.to_string()),
            "radius excludes the far org"
        );

        for (filter, expected) in [
            (
                format!("&types={}", kind.slug.clone().unwrap()),
                vec![near.id],
            ),
            ("&languages=Ukrainian".to_string(), vec![rich_id]),
            ("&open_now=true".to_string(), vec![rich_id]),
            ("&verified=true".to_string(), vec![rich_id]),
            ("&min_rating=4.5".to_string(), vec![rich_id]),
            ("&cost=paid".to_string(), vec![rich_id]),
            ("&added_by=official".to_string(), vec![rich_id]),
        ] {
            let (_, body) = call(&app, Method::GET, &format!("{base}{filter}"), None, None).await;
            let expected: Vec<String> = expected.iter().map(Uuid::to_string).collect();
            assert_eq!(ids(&body), expected, "filter {filter}");
        }

        let (_, verified_first) = call(
            &app,
            Method::GET,
            &format!("{base}&sort=verified"),
            None,
            None,
        )
        .await;
        assert_eq!(verified_first["items"][0]["id"], json!(rich_id));
        let (_, recent) = call(
            &app,
            Method::GET,
            &format!("{base}&sort=recent"),
            None,
            None,
        )
        .await;
        assert_eq!(recent["items"][0]["id"], json!(rich_id), "created last");

        let bbox = format!(
            "/api/organisation/search?min_lat={}&min_lng={}&max_lat={}&max_lng={}",
            lat - 0.001,
            lng - 0.001,
            lat + 0.001,
            lng + 0.001
        );
        let (_, boxed) = call(&app, Method::GET, &bbox, None, None).await;
        assert_eq!(ids(&boxed), vec![near.id.to_string()]);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_search_rejects_bad_parameters() {
        let pool = require_db!();
        let app = test_app!(pool);
        for query in [
            "",
            "lat=10",
            "min_lat=1&min_lng=1&max_lat=2",
            "lat=95&lng=0",
            "min_lat=1&min_lng=1&max_lat=2&max_lng=2&sort=nearest",
            "lat=1&lng=1&sort=popular",
        ] {
            let (status, body) = call(
                &app,
                Method::GET,
                &format!("/api/organisation/search?{query}"),
                None,
                None,
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}: {body}");
        }
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_visit_counter_and_checkins() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let org = ts::org(&pool, OrgSpec::default());

        for expected in [1, 2] {
            let (status, body) = call(
                &app,
                Method::POST,
                &format!("/api/organisation/visit/{}", org.id),
                None,
                None,
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["visitsCount"], expected);
        }

        let checkin = format!("/api/organisation/checkin/{}", org.id);
        let (status, body) = call(
            &app,
            Method::POST,
            &checkin,
            Some(&me.token),
            Some(json!({ "still_active": true, "tip": "Bring your passport" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (_, body) = call(
            &app,
            Method::POST,
            &checkin,
            Some(&me.token),
            Some(json!({ "still_active": false })),
        )
        .await;
        assert_eq!(body["community"]["peopleCame"], 2);
        assert_eq!(body["community"]["stillActivePct"], 50.0);
        assert_eq!(body["community"]["tips"], json!(["Bring your passport"]));

        let pending = ts::org(
            &pool,
            OrgSpec {
                status: Some("pending"),
                ..Default::default()
            },
        );
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/organisation/checkin/{}", pending.id),
            Some(&me.token),
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

/// DB-backed tests of the Google Places location rules (stubbed Google).
#[cfg(test)]
mod location_tests {
    use crate::data::schema::{moderation_events, organisations};
    use crate::error::AppError;
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use crate::utils::google_places::{PlaceDetails, PlaceFuture, PlaceLookup};
    use actix_web::http::{Method, StatusCode};
    use chrono::NaiveDateTime;
    use diesel::prelude::*;
    use serde_json::{Value, json};
    use serial_test::serial;
    use std::sync::Arc;
    use uuid::Uuid;

    const GOOGLE_POINT: (f64, f64) = (52.516275, 13.377704);

    /// Unknown for ids starting with `ChIJunknown`, an outage for `ChIJdown`,
    /// otherwise a permanently closed place at GOOGLE_POINT.
    struct StubPlaces;

    impl PlaceLookup for StubPlaces {
        fn lookup<'a>(&'a self, place_id: &'a str) -> PlaceFuture<'a> {
            Box::pin(async move {
                if place_id.starts_with("ChIJunknown") {
                    return Err(AppError::UnprocessableEntity(
                        json!({ "error": "Unknown google_place_id" }),
                    ));
                }
                if place_id.starts_with("ChIJdown") {
                    return Err(AppError::ServiceUnavailable(json!({ "error": "down" })));
                }
                Ok(PlaceDetails {
                    latitude: GOOGLE_POINT.0,
                    longitude: GOOGLE_POINT.1,
                    business_status: Some("CLOSED_PERMANENTLY".to_string()),
                })
            })
        }
    }

    fn google() -> Option<Arc<dyn PlaceLookup>> {
        Some(Arc::new(StubPlaces))
    }

    fn place_id() -> String {
        format!("ChIJ{}", ts::unique())
    }

    fn stored(
        pool: &crate::utils::db::DbPool,
        id: &Value,
    ) -> (Option<String>, Option<NaiveDateTime>) {
        let id: Uuid = id.as_str().unwrap().parse().unwrap();
        organisations::table
            .find(id)
            .select((
                organisations::location_source,
                organisations::location_refreshed_at,
            ))
            .first(&mut pool.get().unwrap())
            .unwrap()
    }

    fn submitted_flags(pool: &crate::utils::db::DbPool, id: &Value) -> Value {
        let id: Uuid = id.as_str().unwrap().parse().unwrap();
        let flags: Option<Value> = moderation_events::table
            .filter(moderation_events::target_id.eq(id))
            .select(moderation_events::flags)
            .first(&mut pool.get().unwrap())
            .unwrap();
        flags.unwrap()
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_google_place_coordinates_come_from_google() {
        let pool = require_db!();
        let app = test_app!(pool, google());
        let me = ts::user(&pool, "user");

        // The client's point is ignored: the server asks Google.
        let (status, org) = call(&app, Method::POST, "/api/organisation/create", Some(&me.token),
            Some(json!({ "name": "Embassy", "google_place_id": place_id(), "latitude": 1.0, "longitude": 1.0 }))).await;
        assert_eq!(status, StatusCode::OK, "{org}");
        assert_eq!(
            (org["latitude"].as_f64(), org["longitude"].as_f64()),
            (Some(GOOGLE_POINT.0), Some(GOOGLE_POINT.1))
        );
        assert_eq!(org["locationSource"], "google");
        assert!(
            org.get("googleRating").is_none(),
            "Google's rating is never stored or returned"
        );

        let (source, refreshed) = stored(&pool, &org["id"]);
        assert_eq!(source.as_deref(), Some("google"));
        assert!(
            refreshed.is_some(),
            "a verified Google point records when it was fetched"
        );
        let flags = submitted_flags(&pool, &org["id"]);
        assert_eq!(flags["googlePlaceVerified"], true);
        assert_eq!(
            flags["googleBusinessStatus"], "CLOSED_PERMANENTLY",
            "closed places are flagged for the moderator"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_unknown_place_and_google_outage_store_nothing() {
        let pool = require_db!();
        let app = test_app!(pool, google());
        let me = ts::user(&pool, "user");

        for (id, expected) in [
            (
                format!("ChIJunknown{}", ts::unique()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                format!("ChIJdown{}", ts::unique()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            ("../admin".to_string(), StatusCode::UNPROCESSABLE_ENTITY),
        ] {
            let name = format!("Nowhere {}", ts::unique());
            let (status, body) = call(
                &app,
                Method::POST,
                "/api/organisation/create",
                Some(&me.token),
                Some(json!({ "name": name, "google_place_id": id })),
            )
            .await;
            assert_eq!(status, expected, "{id}: {body}");
            let rows: i64 = organisations::table
                .filter(organisations::name.eq(&name))
                .count()
                .get_result(&mut pool.get().unwrap())
                .unwrap();
            assert_eq!(rows, 0, "{id}: nothing is stored");
        }
    }

    /// Without GOOGLE_PLACES_API_KEY the client's point is kept but marked
    /// unverified, like the other optional integrations.
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_without_a_server_key_the_point_is_unverified() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");

        let (status, org) = call(&app, Method::POST, "/api/organisation/create", Some(&me.token),
            Some(json!({ "name": "Unverified", "google_place_id": place_id(), "latitude": 48.85, "longitude": 2.35 }))).await;
        assert_eq!(status, StatusCode::OK, "{org}");
        assert_eq!(org["latitude"].as_f64(), Some(48.85));
        assert_eq!(
            stored(&pool, &org["id"]),
            (Some("google".to_string()), None)
        );
        assert_eq!(
            submitted_flags(&pool, &org["id"])["googlePlaceVerified"],
            false
        );

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": "Bad id", "google_place_id": "id?key=x" })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "place ids are validated even without a key"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_one_organisation_per_google_place() {
        let pool = require_db!();
        let app = test_app!(pool, google());
        let me = ts::user(&pool, "user");
        let id = place_id();

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": "First", "google_place_id": id })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": "Second", "google_place_id": id })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["constraint"], "organisations_google_place_id_key");
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_a_point_is_a_pair_and_user_pins_are_ours() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");

        for body in [
            json!({ "name": "x", "latitude": 10.0 }),
            json!({ "name": "x", "longitude": 10.0 }),
        ] {
            let (status, _) = call(
                &app,
                Method::POST,
                "/api/organisation/create",
                Some(&me.token),
                Some(body.clone()),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        }

        let (_, pinned) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": "Pinned", "latitude": 41.9, "longitude": 12.5 })),
        )
        .await;
        assert_eq!(pinned["locationSource"], "user_pin");
        let (_, online) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&me.token),
            Some(json!({ "name": "Online community" })),
        )
        .await;
        assert_eq!(online["locationSource"], Value::Null);

        let path = format!(
            "/api/organisation/update/{}",
            pinned["id"].as_str().unwrap()
        );
        let (status, _) = call(
            &app,
            Method::PUT,
            &path,
            Some(&me.token),
            Some(json!({ "longitude": 12.6 })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "updates move a point as a pair too"
        );
    }

    /// A point moved by hand is no longer Google's: it is never refreshed.
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_moved_points_stop_being_google_points() {
        let pool = require_db!();
        let app = test_app!(pool, google());
        let (creator, moderator) = (ts::user(&pool, "user"), ts::user(&pool, "moderator"));

        let (_, org) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&creator.token),
            Some(json!({ "name": "Moved", "google_place_id": place_id() })),
        )
        .await;
        let path = format!("/api/organisation/update/{}", org["id"].as_str().unwrap());

        let (status, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&moderator.token),
            Some(json!({ "latitude": 52.5164, "longitude": 13.3778 })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(
            stored(&pool, &org["id"]),
            (Some("moderator".to_string()), None)
        );

        let (_, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&creator.token),
            Some(json!({ "latitude": 52.5165, "longitude": 13.3779 })),
        )
        .await;
        assert_eq!(body["locationSource"], "user_pin");

        // Edits that do not touch the point keep its source.
        let (_, body) = call(
            &app,
            Method::PUT,
            &path,
            Some(&moderator.token),
            Some(json!({ "description": "x" })),
        )
        .await;
        assert_eq!(body["locationSource"], "user_pin");
        ts::org(&pool, OrgSpec::default()); // factory still builds valid rows
    }

    /// Review regression: saving an edit form re-sends the coordinates it
    /// loaded. That must not turn a verified Google point into a user pin
    /// (it would lose its refresh time and drop out of the refresh job).
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_resending_the_same_point_keeps_it_a_google_point() {
        let pool = require_db!();
        let app = test_app!(pool, google());
        let (creator, moderator) = (ts::user(&pool, "user"), ts::user(&pool, "moderator"));
        let (_, org) = call(
            &app,
            Method::POST,
            "/api/organisation/create",
            Some(&creator.token),
            Some(json!({ "name": "Unchanged", "google_place_id": place_id() })),
        )
        .await;
        let before = stored(&pool, &org["id"]);
        assert_eq!(before.0.as_deref(), Some("google"));

        let path = format!("/api/organisation/update/{}", org["id"].as_str().unwrap());
        for who in [&moderator, &creator] {
            let (status, body) = call(&app, Method::PUT, &path, Some(&who.token), Some(json!({
                "description": "edited", "latitude": org["latitude"], "longitude": org["longitude"],
            }))).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(
                stored(&pool, &org["id"]),
                before,
                "same point, same provenance"
            );
        }
    }
}
