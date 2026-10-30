use abyssmarket_server::{connect_database, init_tracing, load_env, Config};
use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use sqlx::PgPool;

#[derive(Clone)]
struct AppState {
    database: PgPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    load_env();
    init_tracing();

    let config = Config::from_env()?;
    let database = connect_database(&config).await?;
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(readiness))
        .with_state(AppState { database });

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("no se pudo escuchar en {}", config.bind_addr))?;
    tracing::info!(address = %config.bind_addr, "API iniciada");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("falló el servidor HTTP")?;

    Ok(())
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readiness(State(state): State<AppState>) -> StatusCode {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.database)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(error) => {
            tracing::warn!(%error, "PostgreSQL no está disponible");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "no se pudo escuchar la señal de apagado");
    }
}
