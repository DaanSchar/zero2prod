use std::fmt::{Display, Formatter};
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Failed to retrieve subscriber id from token")?
        .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&pool, id)
        .await
        .context("Failed to confirm subscriber")?;

    return Ok(HttpResponse::Ok().finish());
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("There is no subscriber associated with this subscription token")]
    UnknownToken,
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnexpectedError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnknownToken => actix_web::http::StatusCode::UNAUTHORIZED,
        }
    }
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}