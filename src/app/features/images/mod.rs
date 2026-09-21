pub mod config;
pub mod controllers;
pub mod entities;
pub mod presenters;
pub mod repositories;
pub mod requests;
pub mod usecases;

use utoipa::OpenApi;

/// Per-feature OpenAPI doc, merged into the root doc in main.rs.
#[derive(OpenApi)]
#[openapi(
    paths(
        controllers::upload_image,
        controllers::delete_image,
        controllers::list_images,
        controllers::fetch_image,
        controllers::set_primary_image,
    ),
    components(schemas(
        presenters::ImageContent,
        presenters::MultipleImagesResponse,
        requests::ImagesListQueryParams,
    )),
    tags((name = "Images", description = "Image upload and management endpoints"))
)]
pub struct ApiDoc;
