use super::entities::Organisation;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;

pub trait OrganisationPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation>;

    fn to_single_json(&self, item: Organisation) -> HttpResponse;

}

#[derive(Clone)]
pub struct OrganisationPresenterImpl {}
impl OrganisationPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl OrganisationPresenter for OrganisationPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    // TODO: Tmp solution
    fn to_single_typed_json(&self, item: Organisation) -> HttpResponse<Organisation> {
        HttpResponse::<Organisation>::with_body(
            StatusCode::OK,
            item
        )
    }
    
    fn to_single_json(&self, item: Organisation) -> HttpResponse {
        HttpResponse::Ok().json(item)
    }
}
