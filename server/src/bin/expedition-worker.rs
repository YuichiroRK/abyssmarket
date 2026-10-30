use abyssmarket_server::{connect_database, init_tracing, load_env, Config};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    init_tracing();

    let config = Config::from_env()?;
    let database = connect_database(&config).await?;
    abyssmarket_server::worker::run(database).await
}
