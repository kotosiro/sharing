pub mod schemas;
use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::share::Entity as ShareEntity;
use crate::server::routers::SharedState;
use crate::server::services::error::Error;
use crate::server::services::share::Share;
use crate::server::utilities::postgres::Utility as PostgresUtility;
use anyhow::anyhow;
use axum::extract::Extension;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminSharesPostRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminSharesPostResponse {
    pub share: Share,
}

#[utoipa::path(
    post,
    path = "/admin/shares",
    request_body = AdminSharesPostRequest,
    responses(
        (status = 201, description = "The share was successfully registered.", body = AdminSharesPostResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 409, description = "The share was already registered.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state, account))]
pub async fn post(
    Extension(account): Extension<AccountEntity>,
    Extension(state): Extension<SharedState>,
    Json(payload): Json<AdminSharesPostRequest>,
) -> Result<Response, Error> {
    let Ok(share) = ShareEntity::new(payload.id, payload.name, payload.description, account.id().to_string()) else {
        tracing::error!("requested share data is malformed");
        return Err(Error::ValidationFailed);
    };
    match PostgresUtility::error(share.save(&state.pg_pool).await)? {
        Ok(_) => {
            tracing::info!("share was successfully registered");
            Ok((
                StatusCode::CREATED,
                Json(AdminSharesPostResponse {
                    share: Share::from(share),
                }),
            )
                .into_response())
        }
        Err(e) if PostgresUtility::is_conflict(&e) => {
            tracing::error!("share was already registered");
            Err(Error::Conflict)
        }
        _ => {
            tracing::error!(
                "request is not handled correctly due to a server error while updating share"
            );
            Err(anyhow!("error occured while updating share").into())
        }
    }
}
