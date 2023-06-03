use crate::config;
use crate::server::entities::account::Entity as Account;
pub use crate::server::middlewares::jwt::Keys as JwtKeys;
use crate::server::utilities::postgres::Utility as PostgresUtility;
use anyhow::anyhow;
use anyhow::Result;
use sqlx::PgPool;

pub struct Utility;

impl Utility {
    pub async fn init_postgres(pool: &PgPool) -> Result<Account> {
        let admin = if let Ok(admin) = Account::new(
            None,
            config::fetch::<String>("admin_name"),
            config::fetch::<String>("admin_email"),
            config::fetch::<String>("admin_password"),
            config::fetch::<String>("admin_namespace"),
            config::fetch::<i64>("admin_ttl"),
            String::from("administrator"),
        ) {
            admin
        } else {
            tracing::error!("admin account data is malformed");
            return Err(anyhow!("failed to validate admin account"));
        };
        match PostgresUtility::error(admin.save(&pool).await)? {
            Ok(_) => {
                tracing::info!("admin account was successfully registered");
                Ok(admin)
            }
            Err(e) if PostgresUtility::is_conflict(&e) => {
                tracing::warn!("admin account was already registered");
                Ok(admin)
            }
            _ => {
                tracing::error!("unknown error occured while creating admin account");
                Err(anyhow!("error occured while updating account"))
            }
        }
    }
}
