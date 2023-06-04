use crate::impl_string_property;
use crate::impl_uuid_property;
use crate::server::entities::account::Id as AccountId;
use crate::server::middlewares::jwt::Role;
use crate::server::repositories::token::Repository;
use anyhow::Result;
use getset::Getters;
use getset::Setters;
use sqlx::postgres::PgQueryResult;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id {
    value: Uuid,
}

impl_uuid_property!(Id);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Email {
    #[validate(email)]
    value: String,
}

impl_string_property!(Email);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Value {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Value);

#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters)]
pub struct Entity {
    #[getset(get = "pub")]
    id: Id,
    #[getset(get = "pub", set = "pub")]
    email: Email,
    #[getset(get = "pub", set = "pub")]
    role: Role,
    #[getset(get = "pub", set = "pub")]
    value: Value,
    #[getset(get = "pub")]
    created_by: AccountId,
}

impl Entity {
    pub fn new(
        id: impl Into<Option<String>>,
        email: String,
        role: String,
        value: String,
        created_by: String,
    ) -> Result<Self> {
        Ok(Self {
            id: Id::try_from(id.into().unwrap_or(uuid::Uuid::new_v4().to_string()))?,
            email: Email::new(email)?,
            role: role.parse()?,
            value: Value::new(value)?,
            created_by: AccountId::try_from(created_by)?,
        })
    }

    pub async fn save(&self, pg_pool: &PgPool) -> Result<PgQueryResult> {
        Repository::upsert(&self, pg_pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_valid_id() {
        assert!(matches!(Id::try_from(testutils::rand::uuid()), Ok(_)));
    }

    #[test]
    fn test_invalid_id() {
        assert!(matches!(Id::try_from(testutils::rand::string(255)), Err(_)));
    }

    #[test]
    fn test_valid_email() {
        assert!(matches!(Email::new(testutils::rand::email()), Ok(_)));
    }

    #[test]
    fn test_invalid_email() {
        assert!(matches!(Email::new(testutils::rand::string(20)), Err(_)));
    }

    #[test]
    fn test_valid_role() {
        let candidates = vec!["administrator", "provider", "recipient"];
        let role = testutils::rand::choose(&candidates);
        assert!(matches!(Role::from_str(role), Ok(_)));
    }

    #[test]
    fn test_invalid_role() {
        let candidates = vec!["apple", "orange", "strawberry", "grape"];
        let role = testutils::rand::choose(&candidates);
        assert!(matches!(Role::from_str(role), Err(_)));
    }

    #[test]
    fn test_valid_value() {
        assert!(matches!(Value::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_value() {
        assert!(matches!(Value::new(""), Err(_)));
    }
}
