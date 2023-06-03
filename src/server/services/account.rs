use crate::server::entities::account::Entity as AccountEntity;
use crate::server::entities::account::Name as AccountName;
use crate::server::middlewares::jwt::Role;
use crate::server::utilities::postgres::PgAcquire;
use anyhow::Context;
use anyhow::Result;
use sqlx::query_builder::QueryBuilder;
use sqlx::Execute;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub email: String,
    pub namespace: String,
    pub ttl: i64,
    pub role: Role,
}

impl Account {
    pub fn from(entity: AccountEntity) -> Self {
        Self {
            name: entity.name().to_string(),
            email: entity.email().to_string(),
            namespace: entity.namespace().to_string(),
            ttl: entity.ttl().to_i64(),
            role: *entity.role(),
        }
    }
}

pub struct Service;

impl Service {
    pub async fn query(
        limit: Option<&i64>,
        after: Option<&AccountName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<Account>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            "SELECT
                 name,
                 email,
                 namespace,
                 ttl,
                 role
             FROM account",
        );
        if let Some(name) = after {
            builder.push(" WHERE name >= ");
            builder.push_bind(name);
        }
        builder.push(" ORDER BY name ");
        if let Some(limit) = limit {
            builder.push(" LIMIT ");
            builder.push_bind(limit);
        }
        let mut query = sqlx::query_as::<_, Account>(builder.build().sql().into());
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<Account> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list accounts from [account]")?;
        Ok(rows)
    }

    pub async fn query_by_name(
        name: &AccountName,
        executor: impl PgAcquire<'_>,
    ) -> Result<Option<Account>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let row: Option<Account> = sqlx::query_as::<_, Account>(
            "SELECT
                 name,
                 email,
                 namespace,
                 ttl,
                 role
             FROM account
             WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&mut *conn)
        .await
        .context(format!(
            r#"failed to select "{}" from [account]"#,
            name.as_str()
        ))?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::entities::account::Entity as AccountEntity;
    use crate::server::repositories::account::Repository as AccountRepository;
    use anyhow::Context;
    use anyhow::Result;
    use sqlx::PgConnection;
    use sqlx::PgPool;
    use std::cmp::min;

    async fn create(tx: &mut PgConnection) -> Result<AccountEntity> {
        let roles = vec![
            String::from("administrator"),
            String::from("provider"),
            String::from("recipient"),
        ];
        let account = AccountEntity::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::email(),
            testutils::rand::string(10),
            testutils::rand::string(10),
            testutils::rand::i64(1, 100000),
            testutils::rand::choose(&roles).to_owned(),
        )
        .context("failed to validate account")?;
        AccountRepository::upsert(&account, tx)
            .await
            .context("failed to create account")?;
        Ok(account)
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_with_default_limit(pool: PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .expect("transaction should be started properly");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create(&mut tx)
                .await
                .expect("new account should be created");
        }
        let fetched = Service::query(None, None, &mut tx)
            .await
            .expect("created account should be listed");
        assert_eq!(records as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_with_specified_limit(pool: PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .expect("transaction should be started properly");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create(&mut tx)
                .await
                .expect("new account should be created");
        }
        let limit = testutils::rand::i64(0, 20);
        let fetched = Service::query(Some(&limit), None, &mut tx)
            .await
            .expect("created account should be listed");
        assert_eq!(min(records, limit) as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_name(pool: PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .expect("transaction should be started properly");
        let account = create(&mut tx)
            .await
            .expect("new account should be created");
        let fetched = Service::query_by_name(&account.name(), &mut tx)
            .await
            .expect("created account should be found");
        if let Some(fetched) = fetched {
            assert_eq!(&fetched.name, account.name().as_str());
            assert_eq!(&fetched.email, account.email().as_str());
            assert_eq!(&fetched.namespace, account.namespace().as_str());
            assert_eq!(&fetched.ttl, account.ttl().as_i64());
            assert_eq!(&fetched.role, account.role());
        } else {
            panic!("created account should be found");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
