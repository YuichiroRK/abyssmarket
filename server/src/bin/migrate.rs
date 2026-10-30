use abyssmarket_server::{connect_database, init_tracing, load_env, run_migrations, Config};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    init_tracing();

    let config = Config::from_env()?;
    let database = connect_database(&config).await?;
    run_migrations(&database).await?;
    tracing::info!("migraciones aplicadas");
    Ok(())
}
