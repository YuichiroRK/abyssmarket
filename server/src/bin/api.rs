use abyssmarket_server::{api::router, connect_database, init_tracing, load_env, Config};
use anyhow::{Context, Result};
use axum::serve;

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    init_tracing();

    let config = Config::from_env()?;
    let database = connect_database(&config).await?;
    let app = router(database, config.expedition_seconds_per_depth);

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("no se pudo escuchar en {}", config.bind_addr))?;
    tracing::info!(address = %config.bind_addr, "API iniciada");

    serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("falló el servidor HTTP")?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "no se pudo escuchar la señal de apagado");
    }
}
