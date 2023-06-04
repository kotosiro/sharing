#![allow(unused)]
use crate::server::entities::schema::Entity;
use crate::server::entities::schema::Id;
use crate::server::entities::schema::Name;
use crate::server::entities::share::Id as ShareId;
use crate::server::entities::table::Id as TableId;
use crate::server::utilities::postgres::PgAcquire;
use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct Row {
    pub id: Uuid,
    pub name: String,
    pub table_id: Uuid,
    pub share_id: Uuid,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Repository;

impl Repository {
    pub async fn upsert(schema: &Entity, executor: impl PgAcquire<'_>) -> Result<PgQueryResult> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        sqlx::query(
            r#"INSERT INTO "schema" (
                   id,
                   name,
                   table_id,
                   share_id,
                   created_by
               ) VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT(id)
               DO UPDATE
               SET name = $2,
                   table_id = $3,
                   share_id = $4,
                   created_by = $5"#,
        )
        .bind(schema.id())
        .bind(schema.name())
        .bind(schema.table_id())
        .bind(schema.share_id())
        .bind(schema.created_by())
        .execute(&mut *conn)
        .await
        .context(format!(
            r#"failed to upsert "{}" into [schema]"#,
            schema.id().as_uuid()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::entities::account::Entity as Account;
    use crate::server::entities::account::Id as AccountId;
    use crate::server::entities::share::Entity as Share;
    use crate::server::entities::table::Entity as Table;
    use crate::server::repositories::account::Repository as AccountRepository;
    use crate::server::repositories::share::Repository as ShareRepository;
    use crate::server::repositories::table::Repository as TableRepository;
    use anyhow::Context;
    use anyhow::Result;
    use sqlx::PgConnection;
    use sqlx::PgPool;
    use std::cmp::min;

    async fn create_account(tx: &mut PgConnection) -> Result<Account> {
        let roles = vec![
            String::from("administrator"),
            String::from("provider"),
            String::from("recipient"),
        ];
        let account = Account::new(
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

    async fn create_table(account_id: &AccountId, tx: &mut PgConnection) -> Result<Table> {
        let table = Table::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::string(10),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate table")?;
        TableRepository::upsert(&table, tx)
            .await
            .context("failed to create table")?;
        Ok(table)
    }

    async fn create_share(account_id: &AccountId, tx: &mut PgConnection) -> Result<Share> {
        let share = Share::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::string(100),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate share")?;
        ShareRepository::upsert(&share, tx)
            .await
            .context("failed to create share")?;
        Ok(share)
    }

    async fn create_schema(
        account_id: &AccountId,
        table_id: &TableId,
        share_id: &ShareId,
        tx: &mut PgConnection,
    ) -> Result<Entity> {
        let schema = Entity::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            table_id.to_uuid().to_string(),
            share_id.to_uuid().to_string(),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate schema")?;
        Repository::upsert(&schema, tx)
            .await
            .context("failed to create schema")?;
        Ok(schema)
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create(pool: PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .expect("transaction should be started properly");
        let account = create_account(&mut tx)
            .await
            .expect("new account should be created");
        let table = create_table(account.id(), &mut tx)
            .await
            .expect("new table should be created");
        let share = create_share(account.id(), &mut tx)
            .await
            .expect("new share should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create_schema(account.id(), table.id(), share.id(), &mut tx)
                .await
                .expect("new schema should be created");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
