use crate::server::entities::schema::Entity as SchemaEntity;
use crate::server::entities::schema::Name as SchemaName;
use crate::server::entities::share::Name as ShareName;
use crate::server::utilities::postgres::PgAcquire;
use anyhow::Context;
use anyhow::Result;
use sqlx::query_builder::QueryBuilder;
use sqlx::Execute;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub id: String,
    pub name: String,
}

impl Schema {
    pub fn from(entity: SchemaEntity) -> Self {
        Self {
            id: entity.id().to_string(),
            name: entity.name().to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDetail {
    pub name: String,
    pub share: String,
}

pub struct Service;

impl Service {
    pub async fn query_by_share_name(
        share_name: &ShareName,
        limit: Option<&i64>,
        after: Option<&SchemaName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<SchemaDetail>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            r#"WITH these_schemas AS (
                   SELECT
                       DISTINCT "schema".name AS name, share.name AS share
                   FROM "schema"
                   LEFT JOIN share ON share.id = "schema".share_id
                   WHERE share.name = "#,
        );
        builder.push_bind(share_name);
        builder.push(
            "
               )
               SELECT
                   name,
                   share
               FROM these_schemas",
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
        let mut query = sqlx::query_as::<_, SchemaDetail>(builder.build().sql().into());
        query = query.bind(share_name);
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<SchemaDetail> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list schemas from [schema]")?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::entities::account::Entity as AccountEntity;
    use crate::server::entities::account::Id as AccountId;
    use crate::server::entities::schema::Entity as SchemaEntity;
    use crate::server::entities::schema::Name as SchemaName;
    use crate::server::entities::share::Entity as ShareEntity;
    use crate::server::entities::share::Id as ShareId;
    use crate::server::entities::table::Entity as TableEntity;
    use crate::server::entities::table::Id as TableId;
    use crate::server::repositories::account::Repository as AccountRepository;
    use crate::server::repositories::schema::Repository as SchemaRepository;
    use crate::server::repositories::share::Repository as ShareRepository;
    use crate::server::repositories::table::Repository as TableRepository;
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
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate share")?;
        ShareRepository::upsert(&share, tx)
            .await
            .context("failed to crate share")?;
        Ok(share)
    }

    async fn create_table(account_id: &AccountId, tx: &mut PgConnection) -> Result<TableEntity> {
        let table = TableEntity::new(
            testutils::rand::uuid(),
            testutils::rand::string(10),
            testutils::rand::string(10),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate table")?;
        TableRepository::upsert(&table, tx)
            .await
            .context("failed to crate table")?;
        Ok(table)
    }

    async fn create_schema(
        schema_name: &SchemaName,
        table_id: &TableId,
        share_id: &ShareId,
        account_id: &AccountId,
        tx: &mut PgConnection,
    ) -> Result<SchemaEntity> {
        let schema = SchemaEntity::new(
            testutils::rand::uuid(),
            schema_name.to_string(),
            table_id.to_uuid().to_string(),
            share_id.to_uuid().to_string(),
            account_id.to_uuid().to_string(),
        )
        .context("failed to validate schema")?;
        SchemaRepository::upsert(&schema, tx)
            .await
            .context("failed to crate schema")?;
        Ok(schema)
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
        let share = create_share(account.id(), &mut tx)
            .await
            .expect("new share should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..testutils::rand::i64(1, 20) {
                let table = create_table(account.id(), &mut tx)
                    .await
                    .expect("new table should be created");
                create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                    .await
                    .expect("new schema should be created");
            }
        }
        let fetched = Service::query_by_share_name(share.name(), None, None, &mut tx)
            .await
            .expect("created schema should be listed");
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
        let share = create_share(account.id(), &mut tx)
            .await
            .expect("new share should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..testutils::rand::i64(1, 20) {
                let table = create_table(account.id(), &mut tx)
                    .await
                    .expect("new table should be created");
                create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                    .await
                    .expect("new schema should be created");
            }
        }
        let limit = testutils::rand::i64(0, 20);
        let fetched = Service::query_by_share_name(share.name(), Some(&limit), None, &mut tx)
            .await
            .expect("created schema should be listed");
        assert_eq!(min(records, limit) as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
