use axum::{extract::{Path, State, Extension}, Json, http::StatusCode};
use serde_json::Value;
use uuid::Uuid;

use crate::db;
use crate::http::error::ApiError;
use crate::http::handlers::HttpState;
use crate::http::mw::AuthenticatedUser;
use crate::http::types::{NotificationResponse, UnreadCountResponse};

fn internal_err(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

fn row_to_response(row: &db::NotificationRow) -> NotificationResponse {
    NotificationResponse {
        id: row.id,
        order_id: row.order_id,
        r#type: row.notification_type.clone(),
        title: row.title.clone(),
        body: row.body.clone(),
        detail: row.detail.clone(),
        read: row.read_at.is_some(),
        created_at: row.created_at,
    }
}

pub async fn list_notifications(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<NotificationResponse>>, ApiError> {
    let rows = db::list_notifications(&state.pool, user.user_id, 50, 0)
        .await
        .map_err(internal_err)?;

    Ok(Json(rows.iter().map(row_to_response).collect()))
}

pub async fn get_unread_count(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<UnreadCountResponse>, ApiError> {
    let count = db::count_unread_notifications(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    Ok(Json(UnreadCountResponse { count }))
}

pub async fn mark_notification_read(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let marked = db::mark_notification_read(&state.pool, id, user.user_id)
        .await
        .map_err(internal_err)?;

    if marked {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub async fn mark_all_notifications_read(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<StatusCode, ApiError> {
    db::mark_all_notifications_read(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    Ok(StatusCode::OK)
}
