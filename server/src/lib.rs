use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_subscriber::EnvFilter;

pub mod api;
pub mod expeditions;
pub mod worker;

pub struct Config {
    pub bind_addr: SocketAddr,
    pub expedition_seconds_per_depth: i64,
    database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
        let bind_addr = bind_addr
            .parse()
            .with_context(|| format!("BIND_ADDR debe ser una dirección válida: {bind_addr}"))?;
        let database_url = env::var("DATABASE_URL").context("falta la variable DATABASE_URL")?;
        let expedition_seconds_per_depth = env::var("EXPEDITION_SECONDS_PER_DEPTH")
            .unwrap_or_else(|_| "60".to_owned())
            .parse::<i64>()
            .context("EXPEDITION_SECONDS_PER_DEPTH debe ser un entero")?;
        if !(1..=86_400).contains(&expedition_seconds_per_depth) {
            anyhow::bail!("EXPEDITION_SECONDS_PER_DEPTH debe estar entre 1 y 86400");
        }

        Ok(Self {
            bind_addr,
            expedition_seconds_per_depth,
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

pub async fn run_migrations(database: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(database)
        .await
        .context("no se pudieron aplicar las migraciones")
}
