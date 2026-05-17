use axum::{Json, extract::State, http::StatusCode, response::{IntoResponse, Response}};

use crate::{
    http::auth,
    http::error::ApiError,
    http::handlers::HttpState,
    http::types::SupportRequest,
};

pub async fn support_contact(
    State(state): State<HttpState>,
    Json(payload): Json<SupportRequest>,
) -> Result<Response, ApiError> {
    let email = payload.email.trim().to_lowercase();
    if !auth::is_valid_email(&email) {
        return Err(ApiError::bad_request("valid email is required"));
    }
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let subject = payload.subject.trim();
    if subject.is_empty() {
        return Err(ApiError::bad_request("subject is required"));
    }
    let message = payload.message.trim();
    if message.len() < 10 {
        return Err(ApiError::bad_request("message must be at least 10 characters"));
    }

    if let Some(ref email_client) = state.email_client {
        email_client
            .send_support_email(&email, name, subject, message)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    } else {
        tracing::warn!(
            email = %email, name = %name, subject = %subject,
            "no email client configured — support request dropped"
        );
    }

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&serde_json::json!({"success": true})).unwrap_or_default(),
    )
        .into_response())
}
