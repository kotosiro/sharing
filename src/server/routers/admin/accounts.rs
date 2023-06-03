use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::account::Name as AccountName;
use crate::server::routers::SharedState;
use crate::server::services::account::Account;
use crate::server::services::account::Service as AccountService;
use crate::server::services::error::Error;
use crate::server::utilities::postgres::Utility as PostgresUtility;
use anyhow::anyhow;
use axum::extract::Extension;
use axum::extract::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use utoipa::IntoParams;
use utoipa::ToSchema;

const DEFAULT_PAGE_RESULTS: usize = 10;

#[derive(Debug, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsPostRequest {
    pub id: Option<String>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub namespace: String,
    pub ttl: i64,
    pub role: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsPostResponse {
    pub account: Account,
}

#[utoipa::path(
    post,
    path = "/admin/accounts",
    request_body = AdminAccountsPostRequest,
    responses(
        (status = 201, description = "The account was successfully registered.", body = AdminAccountsPostResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 409, description = "The account was already registered.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn post(
    Extension(state): Extension<SharedState>,
    Json(payload): Json<AdminAccountsPostRequest>,
) -> Result<Response, Error> {
    let Ok(account) = AccountEntity::new(
	payload.id,
	payload.name,
	payload.email,
	payload.password,
	payload.namespace,
	payload.ttl,
	payload.role,
    ) else {
        tracing::error!("requested account data is malformed");
        return Err(Error::ValidationFailed);
    };
    match PostgresUtility::error(account.save(&state.pg_pool).await)? {
        Ok(_) => {
            tracing::info!("account was successfully registered");
            Ok((
                StatusCode::CREATED,
                Json(AdminAccountsPostResponse {
                    account: Account::from(account),
                }),
            )
                .into_response())
        }
        Err(e) if PostgresUtility::is_conflict(&e) => {
            tracing::error!("account was already registered");
            return Err(Error::Conflict);
        }
        _ => {
            tracing::error!(
                "request is not handled correctly due to a server error while updating account"
            );
            return Err(anyhow!("error occured while updating account").into());
        }
    }
}

#[derive(Debug, serde::Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsGetParams {
    account: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsGetResponse {
    pub account: Account,
}

#[utoipa::path(
    get,
    path = "/admin/accounts/{account}",
    params(
        AdminAccountsGetParams,
    ),
    responses(
        (status = 200, description = "The account's metadata was successfully returned.", body = AdminAccountsGetResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 403, description = "The request is forbidden from being fulfilled.", body = ErrorMessage),
        (status = 404, description = "The requested resource does not exist.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get(
    Extension(state): Extension<SharedState>,
    Path(params): Path<AdminAccountsGetParams>,
) -> Result<Response, Error> {
    let Ok(account) = AccountName::new(params.account) else {
        tracing::error!("requested account data is malformed");
	return Err(Error::ValidationFailed);
    };
    let Ok(account) = AccountService::query_by_name(&account, &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting account");
        return Err(anyhow!("error occured while querying account").into());
    };
    let Some(account) = account else {
        tracing::error!("requested account does not exist");
	return Err(Error::NotFound);
    };
    tracing::info!("account's metadata was successfully returned");
    Ok((StatusCode::OK, Json(AdminAccountsGetResponse { account })).into_response())
}

#[derive(Debug, serde::Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsListQuery {
    pub max_results: Option<i64>,
    pub page_token: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountsListResponse {
    pub items: Vec<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[utoipa::path(
    get,
    path = "/admin/accounts",
    params(
        AdminAccountsListQuery,
    ),
    responses(
        (status = 200, description = "The accounts were successfully returned.", body = AdminAccountsListResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 403, description = "The request is forbidden from being fulfilled.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn list(
    Extension(state): Extension<SharedState>,
    Query(query): Query<AdminAccountsListQuery>,
) -> Result<Response, Error> {
    let limit = if let Some(limit) = &query.max_results {
        let Ok(limit) = usize::try_from(*limit) else {
            tracing::error!("requested limit is malformed");
	    return Err(Error::ValidationFailed);
	};
        limit
    } else {
        DEFAULT_PAGE_RESULTS
    };
    let after = if let Some(name) = &query.page_token {
        AccountName::new(name).ok()
    } else {
        None
    };
    let Ok(accounts) = AccountService::query(Some(&((limit + 1) as i64)), after.as_ref(), &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting accounts");
        return Err(anyhow!("error occured while querying account(s)").into());
    };
    if accounts.len() == limit + 1 {
        let next = &accounts[limit];
        let accounts = &accounts[..limit];
        tracing::info!("accounts were successfully returned");
        return Ok((
            StatusCode::OK,
            Json(AdminAccountsListResponse {
                items: accounts.to_vec(),
                next_page_token: next.name.clone().into(),
            }),
        )
            .into_response());
    }
    tracing::info!("accounts were successfully returned");
    Ok((
        StatusCode::OK,
        Json(AdminAccountsListResponse {
            items: accounts,
            next_page_token: None,
        }),
    )
        .into_response())
}
