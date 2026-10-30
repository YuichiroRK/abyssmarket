use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_subscriber::EnvFilter;

pub struct Config {
    pub bind_addr: SocketAddr,
    database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
        let bind_addr = bind_addr
            .parse()
            .with_context(|| format!("BIND_ADDR debe ser una dirección válida: {bind_addr}"))?;
        let database_url = env::var("DATABASE_URL").context("falta la variable DATABASE_URL")?;

        Ok(Self {
            bind_addr,
            database_url,
        })
    }
}

pub fn load_env() {
    let _ = dotenvy::dotenv();
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

pub async fn connect_database(config: &Config) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .context("no se pudo conectar a PostgreSQL")
}
