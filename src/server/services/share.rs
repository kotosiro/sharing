use crate::server::entities::share::Entity as ShareEntity;
use crate::server::entities::share::Name as ShareName;
use crate::server::utilities::postgres::PgAcquire;
use anyhow::Context;
use anyhow::Result;
use sqlx::query_builder::QueryBuilder;
use sqlx::Execute;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl Share {
    pub fn from(entity: ShareEntity) -> Self {
        Self {
            id: entity.id().to_string(),
            name: entity.name().to_string(),
            description: entity.description().to_string(),
        }
    }
}

pub struct Service;

impl Service {
    pub async fn query(
        limit: Option<&i64>,
        after: Option<&ShareName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<Share>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            "SELECT
                 id::text,
                 name,
                 description
             FROM share",
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
        let mut query = sqlx::query_as::<_, Share>(builder.build().sql().into());
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<Share> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list shares from [share]")?;
        Ok(rows)
    }

    pub async fn query_by_name(
        name: &ShareName,
        executor: impl PgAcquire<'_>,
    ) -> Result<Option<Share>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let row: Option<Share> = sqlx::query_as::<_, Share>(
            "SELECT
                 id::text,
                 name,
                 description
             FROM share
             WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&mut *conn)
        .await
        .context(format!(
            r#"failed to select "{}" from [share]"#,
            name.as_str()
        ))?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::entities::account::Entity as AccountEntity;
    use crate::server::entities::account::Id as AccountId;
    use crate::server::entities::share::Entity as ShareEntity;
    use crate::server::repositories::account::Repository as AccountRepository;
    use crate::server::repositories::share::Repository as ShareRepository;
    use anyhow::Context;
    use anyhow::Result;
    use sqlx::PgConnection;
    use sqlx::PgPool;
    use std::cmp::min;

    async fn create_account(tx: &mut PgConnection) -> Result<AccountEntity> {
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

    async fn create_share(account_id: &AccountId, tx: &mut PgConnection) -> Result<ShareEntity> {
        let share = ShareEntity::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::string(100),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate share")?;
        ShareRepository::upsert(&share, tx)
            .await
            .context("failed to crate share")?;
        Ok(share)
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_with_default_limit(pool: PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .expect("transaction should be started properly");
        let account = create_account(&mut tx)
            .await
            .expect("new account should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create_share(account.id(), &mut tx)
                .await
                .expect("new share should be created");
        }
        let fetched = Service::query(None, None, &mut tx)
            .await
            .expect("created share should be listed");
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
        let account = create_account(&mut tx)
            .await
            .expect("new account should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create_share(account.id(), &mut tx)
                .await
                .expect("new share should be created");
        }
        let limit = testutils::rand::i64(0, 20);
        let fetched = Service::query(Some(&limit), None, &mut tx)
            .await
            .expect("created share should be listed");
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
        let account = create_account(&mut tx)
            .await
            .expect("new account should be created");
        let share = create_share(account.id(), &mut tx)
            .await
            .expect("new share should be created");
        let fetched = Service::query_by_name(&share.name(), &mut tx)
            .await
            .expect("created share should be found");
        if let Some(fetched) = fetched {
            assert_eq!(&fetched.id, share.id().as_uuid().to_string().as_str());
            assert_eq!(&fetched.name, share.name().as_str());
            assert_eq!(&fetched.description, share.description().as_str());
        } else {
            panic!("created account should be found");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
