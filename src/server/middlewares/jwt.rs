use crate::config::JWT_SECRET;
use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::account::Name as AccountName;
use crate::server::routers::SharedState;
use crate::server::services::error::Error;
use anyhow::anyhow;
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::headers::HeaderMapExt;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::decode;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Validation;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub name: String,
    pub email: String,
    pub namespace: String,
    pub role: Role,
    pub exp: i64,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    sqlx::Type,
    strum_macros::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
#[sqlx(type_name = "VARCHAR")]
pub enum Role {
    #[strum(ascii_case_insensitive)]
    Administrator,
    #[strum(ascii_case_insensitive)]
    Provider,
    #[strum(ascii_case_insensitive)]
    Recipient,
}

impl AsRef<str> for Role {
    fn as_ref(&self) -> &str {
        match self {
            Role::Administrator => "administrator",
            Role::Provider => "provider",
            Role::Recipient => "recipient",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Role::Administrator => write!(f, "administrator"),
            Role::Provider => write!(f, "provider"),
            Role::Recipient => write!(f, "recipient"),
        }
    }
}

impl Role {
    pub fn is_recipient(&self) -> bool {
        match self {
            Role::Recipient => true,
            _ => false,
        }
    }
}

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[tracing::instrument(skip(next))]
pub async fn as_administrator<T>(
    mut request: Request<T>,
    next: Next<T>,
) -> std::result::Result<Response, Error>
where
    T: std::fmt::Debug,
{
    let Some(auth) = request.headers().typed_get::<Authorization<Bearer>>() else {
        tracing::error!("bearer token is missing");
	return Err(Error::BadRequest);
    };
    let token = auth.token().to_owned();
    let Ok(jwt) = decode::<Claims>(&token, &JWT_SECRET.decoding, &Validation::default()) else {
        tracing::error!("bearer token cannot be decoded");
        return Err(Error::Unauthorized);
    };
    let Some(state) = request.extensions().get::<SharedState>() else {
        tracing::error!("request is not handled correctly due to a server error while acquiring server state");
        return Err(anyhow!("failed to acquire shared state").into());
    };
    let Ok(name) = AccountName::new(jwt.claims.name.clone()) else {
        tracing::error!("JWT claims' account name is malformed");
	return Err(Error::ValidationFailed);
    };
    let Ok(account) = AccountEntity::load(&name, &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting account");
        return Err(anyhow!("error occured while selecting account from database").into());
    };
    let Some(account) = account else {
        tracing::error!("account was not found");
	return Err(Error::Unauthorized);
    };
    if jwt.claims.role != Role::Administrator {
        tracing::error!("request is forbidden from being fulfilled due to the JWT claims' role");
        return Err(Error::Forbidden);
    }
    request.extensions_mut().insert(account);
    Ok(next.run(request).await)
}

#[tracing::instrument(skip(next))]
pub async fn as_provider<T>(
    mut request: Request<T>,
    next: Next<T>,
) -> std::result::Result<Response, Error>
where
    T: std::fmt::Debug,
{
    let Some(auth) = request.headers().typed_get::<Authorization<Bearer>>() else {
        tracing::error!("bearer token is missing");
	return Err(Error::BadRequest);
    };
    let token = auth.token().to_owned();
    let Ok(jwt) = decode::<Claims>(&token, &JWT_SECRET.decoding, &Validation::default()) else {
        tracing::error!("bearer token cannot be decoded");
        return Err(Error::Unauthorized);
    };
    let Some(state) = request.extensions().get::<SharedState>() else {
        tracing::error!("request is not handled correctly due to a server error while acquiring server state");
        return Err(anyhow!("failed to acquire shared state").into());
    };
    let Ok(name) = AccountName::new(jwt.claims.name.clone()) else {
        tracing::error!("JWT claims' account name is malformed");
	return Err(Error::ValidationFailed);
    };
    let Ok(account) = AccountEntity::load(&name, &state.pg_pool).await else {
        tracing::error!("request is not handled correctly due to a server error while selecting account");
        return Err(anyhow!("error occured while selecting account from database").into());
    };
    let Some(account) = account else {
        tracing::error!("account was not found");
	return Err(Error::Unauthorized);
    };
    if jwt.claims.role != Role::Provider {
        tracing::error!("request is forbidden from being fulfilled due to the JWT claims' role");
        return Err(Error::Forbidden);
    }
    request.extensions_mut().insert(account);
    Ok(next.run(request).await)
}

#[tracing::instrument(skip(next))]
pub async fn as_recipient<T>(
    request: Request<T>,
    next: Next<T>,
) -> std::result::Result<Response, Error>
where
    T: std::fmt::Debug,
{
    let Some(auth) = request.headers().typed_get::<Authorization<Bearer>>() else {
        tracing::error!("bearer token is missing");
	return Err(Error::BadRequest);
    };
    let token = auth.token().to_owned();
    let Ok(jwt) = decode::<Claims>(&token, &JWT_SECRET.decoding, &Validation::default()) else {
        tracing::error!("bearer token cannot be decoded");
        return Err(Error::Unauthorized)?;
    };
    if jwt.claims.role != Role::Recipient {
        tracing::error!("request is forbidden from being fulfilled due to the JWT claims' role");
        return Err(Error::Forbidden);
    }
    Ok(next.run(request).await)
}
