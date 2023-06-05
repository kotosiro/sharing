use crate::server::entities::schema::Name as SchemaName;
use crate::server::entities::share::Name as ShareName;
use crate::server::entities::table::Entity as TableEntity;
use crate::server::entities::table::Name as TableName;
use crate::server::utilities::postgres::PgAcquire;
use anyhow::Context;
use anyhow::Result;
use sqlx::query_builder::QueryBuilder;
use sqlx::Execute;
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub id: String,
    pub name: String,
    pub location: String,
    pub description: String,
}

impl Table {
    pub fn from(entity: TableEntity) -> Self {
        Self {
            id: entity.id().to_string(),
            name: entity.name().to_string(),
            location: entity.location().to_string(),
            description: entity.description().to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TableDetail {
    pub id: String,
    pub share_id: String,
    pub name: String,
    pub schema: String,
    pub share: String,
}

pub struct Service;

impl Service {
    pub async fn query(
        limit: Option<&i64>,
        after: Option<&TableName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<Table>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            r#"SELECT
                   id::text,
                   name,
                   location,
                   description
               FROM "table""#,
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
        let mut query = sqlx::query_as::<_, Table>(builder.build().sql().into());
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<Table> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list tables from [table]")?;
        Ok(rows)
    }

    pub async fn query_by_name(
        name: &TableName,
        executor: impl PgAcquire<'_>,
    ) -> Result<Option<Table>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let row: Option<Table> = sqlx::query_as::<_, Table>(
            r#"SELECT
                   id::text,
                   name,
                   location,
                   description
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

    pub async fn query_by_fqn(
        share_name: &ShareName,
        schema_name: &SchemaName,
        table_name: &TableName,
        executor: impl PgAcquire<'_>,
    ) -> Result<Option<Table>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let row: Option<Table> = sqlx::query_as::<_, Table>(
            r#"SELECT
                   "table".id::text AS id,
                   "table".name AS name,
                   "table".location AS location,
                   "table".description AS description
               FROM "table"
               LEFT JOIN "schema" ON "schema".table_id = "table".id
               LEFT JOIN share ON share.id = "schema".share_id
               WHERE share.name = $1 AND "schema".name = $2 AND "table".name = $3"#,
        )
        .bind(share_name)
        .bind(schema_name)
        .bind(table_name)
        .fetch_optional(&mut *conn)
        .await
        .context(format!(
            r#"failed to select "{}"/"{}"/"{}" from [table]"#,
            share_name.as_str(),
            schema_name.as_str(),
            table_name.as_str(),
        ))?;
        Ok(row)
    }

    pub async fn query_by_share_name(
        share_name: &ShareName,
        limit: Option<&i64>,
        after: Option<&TableName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<TableDetail>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            r#"WITH these_tables AS (
                   SELECT
                       "table".id AS id,
                       share.id AS share_id,
                       "table".name AS name,
                       "schema".name AS schema,
                       share.name AS share
                   FROM "table"
                   LEFT JOIN "schema" ON "schema".table_id = "table".id
                   LEFT JOIN share ON share.id = "schema".share_id
                   WHERE share.name = "#,
        );
        builder.push_bind(share_name);
        builder.push(
            "
               )
               SELECT
                   id::text,
                   share_id::text,
                   name,
                   schema,
                   share
               FROM these_tables",
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
        let mut query = sqlx::query_as::<_, TableDetail>(builder.build().sql().into());
        query = query.bind(share_name);
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<TableDetail> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list tables from [table]")?;
        Ok(rows)
    }

    pub async fn query_by_share_and_schema_name(
        share_name: &ShareName,
        schema_name: &SchemaName,
        limit: Option<&i64>,
        after: Option<&TableName>,
        executor: impl PgAcquire<'_>,
    ) -> Result<Vec<TableDetail>> {
        let mut conn = executor
            .acquire()
            .await
            .context("failed to acquire postgres connection")?;
        let mut builder = QueryBuilder::new(
            r#"WITH these_tables AS (
                   SELECT
                       "table".id AS id,
                       share.id AS share_id,
                       "table".name AS name,
                       "schema".name AS schema,
                       share.name AS share
                   FROM "table"
                   LEFT JOIN "schema" ON "schema".table_id = "table".id
                   LEFT JOIN share ON share.id = "schema".share_id
                   WHERE share.name = "#,
        );
        builder.push_bind(share_name);
        builder.push(r#" AND "schema".name = "#);
        builder.push_bind(schema_name);
        builder.push(
            "
               )
               SELECT
                   id::text,
                   share_id::text,
                   name,
                   schema,
                   share
               FROM these_tables",
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
        let mut query = sqlx::query_as::<_, TableDetail>(builder.build().sql().into());
        query = query.bind(share_name);
        query = query.bind(schema_name);
        if let Some(name) = after {
            query = query.bind(name);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }
        let rows: Vec<TableDetail> = query
            .fetch_all(&mut *conn)
            .await
            .context("failed to list tables from [table]")?;
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
            testutils::rand::string(100),
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
            testutils::rand::string(100),
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
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            create_table(account.id(), &mut tx)
                .await
                .expect("new table should be created");
        }
        let fetched = Service::query(None, None, &mut tx)
            .await
            .expect("created table should be listed");
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
            create_table(account.id(), &mut tx)
                .await
                .expect("new table should be created");
        }
        let limit = testutils::rand::i64(0, 20);
        let fetched = Service::query(Some(&limit), None, &mut tx)
            .await
            .expect("created table should be listed");
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
        let table = create_table(account.id(), &mut tx)
            .await
            .expect("new table should be created");
        let fetched = Service::query_by_name(&table.name(), &mut tx)
            .await
            .expect("created table should be found");
        if let Some(fetched) = fetched {
            assert_eq!(&fetched.id, table.id().as_uuid().to_string().as_str());
            assert_eq!(&fetched.name, table.name().as_str());
            assert_eq!(&fetched.location, table.location().as_str());
            assert_eq!(&fetched.description, table.description().as_str());
        } else {
            panic!("created table should be found");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_fqn(pool: PgPool) -> Result<()> {
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
        let schema_name = SchemaName::new(testutils::rand::string(10))
            .expect("new schema name should be created");
        let table = create_table(account.id(), &mut tx)
            .await
            .expect("new table should be created");
        create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
            .await
            .expect("new schema should be created");
        let fetched = Service::query_by_fqn(share.name(), &schema_name, table.name(), &mut tx)
            .await
            .expect("created table should be found");
        if let Some(fetched) = fetched {
            assert_eq!(&fetched.id, table.id().as_uuid().to_string().as_str());
            assert_eq!(&fetched.name, table.name().as_str());
            assert_eq!(&fetched.location, table.location().as_str());
            assert_eq!(&fetched.description, table.description().as_str());
        } else {
            panic!("created table should be found");
        }
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_share_name_with_default_limit(pool: PgPool) -> Result<()> {
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
        let num_schemas = testutils::rand::i64(0, 20);
        let num_tables = testutils::rand::i64(0, 20);
        for _ in 0..num_schemas {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..num_tables {
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
            .expect("created table should be listed");
        assert_eq!((num_schemas * num_tables) as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_share_name_with_specified_limit(pool: PgPool) -> Result<()> {
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
        let num_schemas = testutils::rand::i64(0, 20);
        let num_tables = testutils::rand::i64(0, 20);
        for _ in 0..num_schemas {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..num_tables {
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
        assert_eq!(min(num_schemas * num_tables, limit) as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_share_and_schema_name_with_default_limit(
        pool: PgPool,
    ) -> Result<()> {
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
        let schema_name = SchemaName::new(testutils::rand::string(10))
            .expect("new schema name should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            let table = create_table(account.id(), &mut tx)
                .await
                .expect("new table should be created");
            create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                .await
                .expect("new schema should be created");
        }
        for _ in 0..testutils::rand::i64(0, 20) {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..records {
                let table = create_table(account.id(), &mut tx)
                    .await
                    .expect("new table should be created");
                create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                    .await
                    .expect("new schema should be created");
            }
        }
        let fetched = Service::query_by_share_and_schema_name(
            share.name(),
            &schema_name,
            None,
            None,
            &mut tx,
        )
        .await
        .expect("created table should be listed");
        assert_eq!(records as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }

    #[sqlx::test]
    #[ignore] // NOTE: Be sure '$ docker compose -f devops/local/docker-compose.yaml up' before running this test
    async fn test_create_and_query_by_share_and_schema_name_with_specified_limit(
        pool: PgPool,
    ) -> Result<()> {
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
        let schema_name = SchemaName::new(testutils::rand::string(10))
            .expect("new schema name should be created");
        let records = testutils::rand::i64(0, 20);
        for _ in 0..records {
            let table = create_table(account.id(), &mut tx)
                .await
                .expect("new table should be created");
            create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                .await
                .expect("new schema should be created");
        }
        for _ in 0..testutils::rand::i64(0, 20) {
            let schema_name = SchemaName::new(testutils::rand::string(10))
                .expect("new schema name should be created");
            for _ in 0..records {
                let table = create_table(account.id(), &mut tx)
                    .await
                    .expect("new table should be created");
                create_schema(&schema_name, table.id(), share.id(), account.id(), &mut tx)
                    .await
                    .expect("new schema should be created");
            }
        }
        let limit = testutils::rand::i64(0, 20);
        let fetched = Service::query_by_share_and_schema_name(
            share.name(),
            &schema_name,
            Some(&limit),
            None,
            &mut tx,
        )
        .await
        .expect("created schema should be listed");
        assert_eq!(min(records, limit) as usize, fetched.len());
        tx.rollback()
            .await
            .expect("rollback should be done properly");
        Ok(())
    }
}
