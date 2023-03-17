mod entities;
mod interactors;
mod repositories;
mod services;
use crate::utils;
use anyhow::Context;
use anyhow::Result;
use redis::Client;
use sqlx::PgPool;

pub struct Server {
    pg_pool: PgPool,
    redis_client: Client,
}

impl Server {
    pub async fn new() -> Result<Self> {
        let pg_pool = utils::new_pg_pool()
            .await
            .context("failed to create postgres connection pool")?;
        let redis_client = utils::new_redis_client().context("failed to create redis client")?;
        Ok(Server {
            pg_pool,
            redis_client,
        })
    }

    pub async fn start(self: Self) -> Result<()> {
        interactors::bind(self.pg_pool, self.redis_client)
            .await
            .context("failed to start API server")
    }
}
