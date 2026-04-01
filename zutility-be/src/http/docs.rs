use axum::{Json, response::Html};
use utoipa::OpenApi;

use crate::{
    domain::order::OrderStatus,
    http::{
        error::ErrorEnvelope,
        handlers,
        types::{
            CancelOrderResponse, CreateOrderRequest, CreateOrderResponse, OrderStatusResponse,
            RateResponse, UtilityItem, UtilityValidateResponse,
        },
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::create_order,
        handlers::get_order,
        handlers::cancel_order,
        handlers::get_current_rate,
        handlers::list_utilities,
        handlers::validate_utility_reference,
        handlers::health_live,
    ),
    components(
        schemas(
            CreateOrderRequest,
            CreateOrderResponse,
            OrderStatusResponse,
            CancelOrderResponse,
            RateResponse,
            UtilityItem,
            UtilityValidateResponse,
            ErrorEnvelope,
            OrderStatus,
        )
    ),
    tags(
        (name = "zutility", description = "Zutility backend API")
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

pub async fn docs_ui() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html>
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <title>Zutility API Docs</title>
    <link rel=\"stylesheet\" href=\"https://unpkg.com/swagger-ui-dist@5/swagger-ui.css\" />
  </head>
  <body>
    <div id=\"swagger-ui\"></div>
    <script src=\"https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js\"></script>
    <script>
      window.ui = SwaggerUIBundle({
        url: '/ops/openapi.json',
        dom_id: '#swagger-ui',
        deepLinking: true,
      });
    </script>
  </body>
</html>
"#,
    )
}
