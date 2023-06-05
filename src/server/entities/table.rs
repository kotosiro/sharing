use crate::impl_string_property;
use crate::impl_uuid_property;
use crate::server::entities::account::Id as AccountId;
use crate::server::repositories::table::Repository;
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
pub struct Name {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Name);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Location {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Location);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Description {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Description);

#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters)]
pub struct Entity {
    #[getset(get = "pub")]
    id: Id,
    #[getset(get = "pub", set = "pub")]
    name: Name,
    #[getset(get = "pub", set = "pub")]
    location: Location,
    #[getset(get = "pub", set = "pub")]
    description: Description,
    #[getset(get = "pub")]
    created_by: AccountId,
}

impl Entity {
    pub fn new(
        id: impl Into<Option<String>>,
        name: String,
        location: String,
        description: String,
        created_by: String,
    ) -> Result<Self> {
        Ok(Self {
            id: Id::try_from(id.into().unwrap_or(uuid::Uuid::new_v4().to_string()))?,
            name: Name::new(name)?,
            location: Location::new(location)?,
            description: Description::new(description)?,
            created_by: AccountId::try_from(created_by)?,
        })
    }

    pub async fn load(name: &Name, pg_pool: &PgPool) -> Result<Option<Self>> {
        match Repository::select_by_name(&name, pg_pool).await? {
            Some(row) => Ok(Self {
                id: Id::new(row.id),
                name: Name::new(row.name)?,
                location: Location::new(row.location)?,
                description: Description::new(row.description)?,
                created_by: AccountId::new(row.created_by),
            }
            .into()),
            _ => Ok(None),
        }
    }

    pub async fn save(&self, pg_pool: &PgPool) -> Result<PgQueryResult> {
        Repository::upsert(&self, pg_pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_id() {
        assert!(matches!(Id::try_from(testutils::rand::uuid()), Ok(_)));
    }

    #[test]
    fn test_invalid_id() {
        assert!(matches!(Id::try_from(testutils::rand::string(255)), Err(_)));
    }

    #[test]
    fn test_valid_name() {
        assert!(matches!(Name::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_name() {
        assert!(matches!(Name::new(""), Err(_)));
    }

    #[test]
    fn test_valid_location() {
        assert!(matches!(Location::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_location() {
        assert!(matches!(Location::new(""), Err(_)));
    }

    #[test]
    fn test_valid_description() {
        assert!(matches!(Name::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_description() {
        assert!(matches!(Name::new(""), Err(_)));
    }
}
