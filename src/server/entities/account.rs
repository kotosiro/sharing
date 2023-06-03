use crate::impl_i64_property;
use crate::impl_string_property;
use crate::impl_uuid_property;
use crate::server::middlewares::jwt::Role;
use crate::server::repositories::account::Repository;
use anyhow::anyhow;
use anyhow::Result;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::PasswordHash;
use argon2::password_hash::PasswordHasher;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::SaltString;
use argon2::Argon2;
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
pub struct Email {
    #[validate(email)]
    value: String,
}

impl_string_property!(Email);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Password {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Password);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Namespace {
    #[validate(length(min = 1))]
    value: String,
}

impl_string_property!(Namespace);

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct Ttl {
    #[validate(range(min = 0))]
    value: i64,
}

impl_i64_property!(Ttl);

#[derive(Debug, Clone, PartialEq, Eq, Getters, Setters)]
pub struct Entity {
    #[getset(get = "pub")]
    id: Id,
    #[getset(get = "pub", set = "pub")]
    name: Name,
    #[getset(get = "pub", set = "pub")]
    email: Email,
    #[getset(get = "pub", set = "pub")]
    password: Password,
    #[getset(get = "pub", set = "pub")]
    namespace: Namespace,
    #[getset(get = "pub", set = "pub")]
    ttl: Ttl,
    #[getset(get = "pub", set = "pub")]
    role: Role,
}

fn hash(password: &[u8]) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed = Argon2::default()
        .hash_password(password, &salt)
        .map_err(|_| anyhow!("falield to hash password"))?;
    Ok(hashed.to_string())
}

fn verify(password: &[u8], hash: &str) -> Result<()> {
    let parsed =
        PasswordHash::new(hash).map_err(|_| anyhow!("falield to parse hashed password"))?;
    Argon2::default()
        .verify_password(password, &parsed)
        .map_err(|_| anyhow!("falield to verify password"))
}

impl Entity {
    pub fn new(
        id: impl Into<Option<String>>,
        name: String,
        email: String,
        password: String,
        namespace: String,
        ttl: i64,
        role: String,
    ) -> Result<Self> {
        Ok(Self {
            id: Id::try_from(id.into().unwrap_or(uuid::Uuid::new_v4().to_string()))?,
            name: Name::new(name)?,
            email: Email::new(email)?,
            password: Password::new(self::hash(password.as_bytes()).unwrap())?,
            namespace: Namespace::new(namespace)?,
            ttl: Ttl::new(ttl)?,
            role: role.parse()?,
        })
    }

    pub async fn load(name: &Name, pg_pool: &PgPool) -> Result<Option<Self>> {
        match Repository::select_by_name(&name, pg_pool).await? {
            Some(row) => Ok(Self {
                id: Id::new(row.id),
                name: Name::new(row.name)?,
                email: Email::new(row.email)?,
                password: Password::new(row.password)?,
                namespace: Namespace::new(row.namespace)?,
                ttl: Ttl::new(row.ttl)?,
                role: row.role.parse()?,
            }
            .into()),
            _ => Ok(None),
        }
    }

    pub async fn save(&self, pg_pool: &PgPool) -> Result<PgQueryResult> {
        Repository::upsert(&self, pg_pool).await
    }

    pub fn verify(&self, password: &[u8]) -> Result<()> {
        self::verify(password.into(), self.password().as_str())
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
    fn test_valid_name() {
        assert!(matches!(Name::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_name() {
        assert!(matches!(Name::new(""), Err(_)));
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
    fn test_valid_password() {
        assert!(matches!(Password::new(testutils::rand::string(255)), Ok(_)));
    }

    #[test]
    fn test_invalid_password() {
        assert!(matches!(Password::new(""), Err(_)));
    }

    #[test]
    fn test_valid_namespace() {
        assert!(matches!(
            Namespace::new(testutils::rand::string(255)),
            Ok(_)
        ));
    }

    #[test]
    fn test_invalid_namespace() {
        assert!(matches!(Namespace::new(""), Err(_)));
    }

    #[test]
    fn test_valid_ttl() {
        assert!(matches!(Ttl::new(testutils::rand::i64(0, 100000)), Ok(_)));
    }

    #[test]
    fn test_invalid_ttl() {
        assert!(matches!(
            Ttl::new(testutils::rand::i64(-100000, -1)),
            Err(_)
        ));
    }

    #[test]
    fn test_valid_role() {
        let candidates = vec!["administrator", "provider", "recipient"];
        let role = testutils::rand::choose(&candidates);
        assert!(matches!(Role::from_str(role), Ok(_)));
    }

    //    #[test]
    //    fn test_invalid_role() {
    //        assert!(matches!(
    //            Ttl::new(testutils::rand::i64(-100000, -1)),
    //            Err(_)
    //        ));
    //    }
}
