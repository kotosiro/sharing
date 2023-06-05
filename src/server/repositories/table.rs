use crate::server::entities::table::Entity;
use crate::server::entities::table::Name;
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
    pub location: String,
    pub description: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Repository;

impl Repository {
    pub async fn upsert(table: &Entity, executor: impl PgAcquire<'_>) -> Result<PgQueryResult> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        sqlx::query(
            r#"INSERT INTO "table" (
                   id,
                   name,
                   location,
                   description,
                   created_by
               ) VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT(id)
               DO UPDATE
               SET name = $2,
                   location = $3,
                   description = $4,
                   created_by = $5"#,
        )
        .bind(table.id())
        .bind(table.name())
        .bind(table.location())
        .bind(table.description())
        .bind(table.created_by())
        .execute(&mut *conn)
        .await
        .context(format!(
            r#"failed to upsert "{}" into [table]"#,
            table.id().as_uuid()
        ))
    }

    pub async fn select_by_name(name: &Name, executor: impl PgAcquire<'_>) -> Result<Option<Row>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let row: Option<Row> = sqlx::query_as::<_, Row>(
            r#"SELECT
                   id,
                   name,
                   location,
                   description,
                   created_by,
                   created_at,
                   updated_at
               FROM "table"
               WHERE name = $1"#,
        )
        .bind(name)
        .fetch_optional(&mut *conn)
        .await
        .context(format!(
            r#"failed to select "{}" from [table]"#,
            name.as_str()
        ))?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::entities::account::Entity as Account;
    use crate::server::entities::account::Id as AccountId;
    use crate::server::repositories::account::Repository as AccountRepository;
    use anyhow::Context;
    use anyhow::Result;
    use sqlx::PgConnection;
    use sqlx::PgPool;

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

    async fn create_table(account_id: &AccountId, tx: &mut PgConnection) -> Result<Entity> {
        let table = Entity::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::string(10),
            testutils::rand::string(100),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate table")?;
        Repository::upsert(&table, tx)
            .await
            .context("failed to create table")?;
        Ok(table)
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_select_by_name(pool: PgPool) -> Result<()> {
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
        let fetched = Repository::select_by_name(&table.name(), &mut tx)
            .await
            .expect("created table should be found");
        if let Some(fetched) = fetched {
            assert_eq!(&fetched.id, table.id().as_uuid());
            assert_eq!(&fetched.name, table.name().as_str());
            assert_eq!(&fetched.location, table.location().as_str());
            assert_eq!(&fetched.description, table.description().as_str());
            assert_eq!(&fetched.created_by, table.created_by().as_uuid());
        } else {
            panic!("created table should be matched");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
