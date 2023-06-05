use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::table::Entity as TableEntity;
use crate::server::entities::table::Name as TableName;
use crate::server::routers::SharedState;
use crate::server::services::error::Error;
use crate::server::services::table::Service as TableService;
use crate::server::services::table::Table;
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
pub struct AdminTablesPostRequest {
    pub id: Option<String>,
    pub name: String,
    pub location: String,
    pub description: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminTablesPostResponse {
    pub table: Table,
}

#[utoipa::path(
    post,
    path = "/admin/tables",
    request_body = AdminTablesPostRequest,
    responses(
        (status = 201, description = "The table was successfully registered.", body = AdminTablesPostResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 409, description = "The table was already registered.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state, account))]
pub async fn post(
    Extension(account): Extension<AccountEntity>,
    Extension(state): Extension<SharedState>,
    Json(payload): Json<AdminTablesPostRequest>,
) -> Result<Response, Error> {
    let Ok(table) = TableEntity::new(payload.id, payload.name, payload.location, payload.description, account.id().to_string()) else {
        tracing::error!("requested table data is malformed");
        return Err(Error::ValidationFailed);
    };
    match PostgresUtility::error(table.save(&state.pg_pool).await)? {
        Ok(_) => {
            tracing::info!("table was successfully registered");
            Ok((
                StatusCode::CREATED,
                Json(AdminTablesPostResponse {
                    table: Table::from(table),
                }),
            )
                .into_response())
        }
        Err(e) if PostgresUtility::is_conflict(&e) => {
            tracing::error!("table was already registered");
            Err(Error::Conflict)
        }
        _ => {
            tracing::error!(
                "request is not handled correctly due to a server error while updating table"
            );
            Err(anyhow!("error occured while updating table").into())
        }
    }
}

#[derive(Debug, serde::Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AdminTablesGetParams {
    table: String,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminTablesGetResponse {
    pub table: Table,
}

#[utoipa::path(
    get,
    path = "/admin/tables/{table}",
    params(
        AdminTablesGetParams,
    ),
    responses(
        (status = 200, description = "The table's metadata was successfully returned.", body = AdminTablesGetResponse),
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
    Path(params): Path<AdminTablesGetParams>,
) -> Result<Response, Error> {
    let Ok(table) = TableName::new(params.table) else {
        tracing::error!("requested table data is malformed");
	return Err(Error::ValidationFailed);
    };
    let Ok(table) = TableService::query_by_name(&table, &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting table");
        return Err(anyhow!("error occured while selecting table").into());
    };
    let Some(table) = table else {
        tracing::error!("requested table does not exist");
	return Err(Error::NotFound);
    };
    tracing::info!("table's metadata was successfully returned");
    Ok((
        StatusCode::OK,
        Json(AdminTablesGetResponse { table: table }),
    )
        .into_response())
}

#[derive(Debug, serde::Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AdminTablesListQuery {
    pub max_results: Option<i64>,
    pub page_token: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminTablesListResponse {
    pub items: Vec<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[utoipa::path(
    get,
    path = "/admin/tables",
    params(
        AdminTablesListQuery,
    ),
    responses(
        (status = 200, description = "The tables were successfully returned.", body = AdminTablesListResponse),
        (status = 400, description = "The request is malformed.", body = ErrorMessage),
        (status = 401, description = "The request is unauthenticated. The bearer token is missing or incorrect.", body = ErrorMessage),
        (status = 403, description = "The request is forbidden from being fulfilled.", body = ErrorMessage),
        (status = 500, description = "The request is not handled correctly due to a server error.", body = ErrorMessage),
    )
)]
#[tracing::instrument(skip(state))]
pub async fn list(
    Extension(state): Extension<SharedState>,
    Query(query): Query<AdminTablesListQuery>,
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
        TableName::new(name).ok()
    } else {
        None
    };
    let Ok(tables) = TableService::query(Some(&((limit + 1) as i64)), after.as_ref(), &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting tables");
        return Err(anyhow!("error occured while selecting table(s)").into());
    };
    if tables.len() == limit + 1 {
        let next = &tables[limit];
        let tables = &tables[..limit];
        tracing::info!("tables were successfully returned");
        return Ok((
            StatusCode::OK,
            Json(AdminTablesListResponse {
                items: tables.to_vec(),
                next_page_token: next.name.clone().into(),
            }),
        )
            .into_response());
    }
    tracing::info!("tables were successfully returned");
    Ok((
        StatusCode::OK,
        Json(AdminTablesListResponse {
            items: tables,
            next_page_token: None,
        }),
    )
        .into_response())
}
