pub mod accounts;
pub mod shares;
pub mod tables;
use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::account::Name as AccountName;
use crate::server::entities::token::Entity as TokenEntity;
use crate::server::middlewares::jwt::Role;
use crate::server::routers::SharedState;
use crate::server::services::error::Error;
use crate::server::services::profile::Profile;
use crate::server::services::profile::Service as ProfileService;
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
pub struct AdminLoginRequest {
    pub account: String,
    pub password: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminLoginResponse {
    pub profile: Profile,
}

#[utoipa::path(
    post,
    path = "/admin/login",
    request_body = AdminLoginRequest,
    responses(
        (status = 200, description = "The profile was successfully returned.", body = AdminLoginResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn login(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<AdminLoginRequest>,
) -> Result<Response, Error> {
    let Ok(account) = AccountName::new(payload.account) else {
        tracing::error!("requested account data is malformed");
        return Err(Error::ValidationFailed);
    };
    let Ok(account) = AccountEntity::load(&account, &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting account");
        return Err(anyhow!("error occured while selecting account from database").into());
    };
    let Some(account) = account else {
        tracing::error!("account does not exist");
        return Err(Error::Unauthorized);
    };
    let Ok(_) = account.verify(payload.password.as_bytes()) else {
        tracing::error!("password is incorrect");
        return Err(Error::Unauthorized);
    };
    let Ok(profile) = ProfileService::issue(
        account.name().to_string(),
        account.email().to_string(),
        account.namespace().to_string(),
        Role::Administrator,
        account.ttl().to_i64(),
    ) else {
        tracing::error!("request is not handled correctly due to a server error while creating profile");
        return Err(anyhow!("failed to create profile").into());
    };
    let Ok(token) = TokenEntity::new(
	None,
	account.email().to_string(),
        Role::Administrator,
	profile.bearer_token.clone(),
	account.id().to_string(),
    ) else {
        tracing::error!("request is not handled correctly due to a server error while creating token");
        return Err(anyhow!("failed to create token").into());
    };
    match PostgresUtility::error(token.save(&state.pg_pool).await)? {
        Ok(_) => {
            tracing::info!("token was successfully registered");
        }
        Err(e) if PostgresUtility::is_conflict(&e) => {
            tracing::error!("token was already registered");
        }
        _ => {
            tracing::error!(
                "request is not handled correctly due to a server error while updating token"
            );
            return Err(anyhow!("error occured while updating account").into());
        }
    }
    tracing::info!("profile was successfully returned");
    Ok((
        StatusCode::OK,
        Json(AdminLoginResponse { profile: profile }),
    )
        .into_response())
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminProfileResponse {
    pub profile: Profile,
}

#[utoipa::path(
    get,
    path = "/admin/profile",
    responses(
        (status = 200, description = "The profile were successfully returned.", body = AdminProfileResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 403, description = "The request is forbidden from being fulfilled.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(account))]
pub async fn profile(Extension(account): Extension<AccountEntity>) -> Result<Response, Error> {
    let Ok(profile) = ProfileService::issue(
        account.name().to_string(),
        account.email().to_string(),
        account.namespace().to_string(),
        Role::Recipient,
        account.ttl().to_i64(),
    ) else {
        tracing::error!("request is not handled correctly due to a server error while creating profile");
        return Err(anyhow!("failed to create profile").into());
    };
    tracing::info!("profile was successfully returned");
    Ok((
        StatusCode::OK,
        Json(AdminProfileResponse { profile: profile }),
    )
        .into_response())
}
