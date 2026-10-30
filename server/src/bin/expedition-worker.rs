use abyssmarket_server::{connect_database, init_tracing, load_env, Config};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    init_tracing();

    let config = Config::from_env()?;
    let _database = connect_database(&config).await?;
    tracing::info!("worker de expediciones iniciado; resolución de expediciones pendiente");

    tokio::signal::ctrl_c().await?;
    tracing::info!("worker de expediciones detenido");
    Ok(())
}
