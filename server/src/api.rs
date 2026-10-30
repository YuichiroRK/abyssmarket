use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{connect_info::ConnectInfo, Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use tokio::sync::Semaphore;

const AUTH_WINDOW: Duration = Duration::from_secs(60);
const AUTH_ATTEMPTS_PER_WINDOW: usize = 10;
const MAX_TRACKED_CLIENTS: usize = 1024;
const SESSION_LIFETIME_SECONDS: u64 = 30 * 24 * 60 * 60;

#[derive(Clone)]
struct AppState {
    database: PgPool,
    expedition_seconds_per_depth: i64,
    password_jobs: Arc<Semaphore>,
    rate_limiter: RateLimiter,
}

#[derive(Clone, Default)]
struct RateLimiter {
    attempts: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
}

impl RateLimiter {
    fn allow(&self, address: IpAddr) -> bool {
        let Ok(mut attempts) = self.attempts.lock() else {
            return false;
        };
        let now = Instant::now();

        if !attempts.contains_key(&address) && attempts.len() >= MAX_TRACKED_CLIENTS {
            attempts.retain(|_, timestamps| {
                timestamps
                    .back()
                    .is_some_and(|last| now.duration_since(*last) < AUTH_WINDOW)
            });
            if attempts.len() >= MAX_TRACKED_CLIENTS {
                return false;
            }
        }

        let timestamps = attempts.entry(address).or_default();
        while timestamps
            .front()
            .is_some_and(|first| now.duration_since(*first) >= AUTH_WINDOW)
        {
            timestamps.pop_front();
        }

        if timestamps.len() >= AUTH_ATTEMPTS_PER_WINDOW {
            return false;
        }
        timestamps.push_back(now);
        true
    }
}

pub fn router(database: PgPool, expedition_seconds_per_depth: i64) -> Router {
    let state = AppState {
        database,
        expedition_seconds_per_depth,
        password_jobs: Arc::new(Semaphore::new(2)),
        rate_limiter: RateLimiter::default(),
    };

    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(readiness))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", axum::routing::delete(logout))
        .route("/api/creatures", post(create_creature))
        .route("/api/expeditions", post(create_expedition))
        .route("/api/expeditions/{expedition_id}", get(get_expedition))
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .with_state(state)
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

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("La solicitud no es válida: {0}")]
    Validation(&'static str),
    #[error("Se requiere una sesión válida")]
    Unauthorized,
    #[error("No se encontró el recurso")]
    NotFound,
    #[error("Conflicto: {0}")]
    Conflict(&'static str),
    #[error("Demasiados intentos; espera un minuto")]
    RateLimited,
    #[error("Error interno del servidor")]
    Internal,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(ErrorResponse {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

fn database_error(error: sqlx::Error, conflict_message: &'static str) -> ApiError {
    if error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
    {
        ApiError::Conflict(conflict_message)
    } else {
        tracing::error!(%error, "falló una consulta de API");
        ApiError::Internal
    }
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct PlayerResponse {
    id: i64,
    username: String,
}

#[derive(Serialize)]
struct SessionResponse {
    player: PlayerResponse,
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
}

fn validate_username(username: &str) -> bool {
    (3..=24).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_password(password: &str) -> bool {
    (12..=128).contains(&password.len())
}

async fn register(
    State(state): State<AppState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    Json(credentials): Json<Credentials>,
) -> Result<impl IntoResponse, ApiError> {
    if !state.rate_limiter.allow(address.ip()) {
        return Err(ApiError::RateLimited);
    }
    if !validate_username(&credentials.username) {
        return Err(ApiError::Validation(
            "el usuario debe tener 3–24 caracteres: minúsculas, números o guion bajo",
        ));
    }
    if !validate_password(&credentials.password) {
        return Err(ApiError::Validation(
            "la contraseña debe tener 12–128 bytes",
        ));
    }

    let password_hash = hash_password(&state, credentials.password).await?;
    let (access_token, token_hash) = new_session_token()?;
    let mut transaction = state
        .database
        .begin()
        .await
        .map_err(|error| database_error(error, "el nombre de usuario ya está registrado"))?;
    let player_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO players (username, password_hash) VALUES ($1, $2) RETURNING id",
    )
    .bind(&credentials.username)
    .bind(password_hash)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| database_error(error, "el nombre de usuario ya está registrado"))?;

    sqlx::query(
        "INSERT INTO sessions (token_hash, player_id, expires_at) \
         VALUES ($1, $2, now() + interval '30 days')",
    )
    .bind(token_hash)
    .bind(player_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| database_error(error, "no se pudo crear la sesión"))?;
    transaction
        .commit()
        .await
        .map_err(|error| database_error(error, "no se pudo crear la cuenta"))?;

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            player: PlayerResponse {
                id: player_id,
                username: credentials.username,
            },
            access_token,
            token_type: "Bearer",
            expires_in: SESSION_LIFETIME_SECONDS,
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<SessionResponse>, ApiError> {
    if !state.rate_limiter.allow(address.ip()) {
        return Err(ApiError::RateLimited);
    }
    if !validate_username(&credentials.username) || !validate_password(&credentials.password) {
        return Err(ApiError::Unauthorized);
    }

    let player = sqlx::query("SELECT id, username, password_hash FROM players WHERE username = $1")
        .bind(&credentials.username)
        .fetch_optional(&state.database)
        .await
        .map_err(|error| database_error(error, "nombre de usuario o contraseña incorrectos"))?;
    let Some(player) = player else {
        // Mantén un coste de hash parecido al de verificar una cuenta existente.
        hash_password(&state, credentials.password).await?;
        return Err(ApiError::Unauthorized);
    };
    let player_id: i64 = player.try_get("id").map_err(|error| {
        tracing::error!(%error, "no se pudo leer el id de la cuenta");
        ApiError::Internal
    })?;
    let username: String = player.try_get("username").map_err(|error| {
        tracing::error!(%error, "no se pudo leer el nombre de la cuenta");
        ApiError::Internal
    })?;
    let password_hash: String = player.try_get("password_hash").map_err(|error| {
        tracing::error!(%error, "no se pudo leer el hash de la cuenta");
        ApiError::Internal
    })?;

    if !verify_password(&state, password_hash, credentials.password).await? {
        return Err(ApiError::Unauthorized);
    }

    let (access_token, token_hash) = new_session_token()?;
    sqlx::query(
        "INSERT INTO sessions (token_hash, player_id, expires_at) \
         VALUES ($1, $2, now() + interval '30 days')",
    )
    .bind(token_hash)
    .bind(player_id)
    .execute(&state.database)
    .await
    .map_err(|error| database_error(error, "no se pudo crear la sesión"))?;

    Ok(Json(SessionResponse {
        player: PlayerResponse {
            id: player_id,
            username,
        },
        access_token,
        token_type: "Bearer",
        expires_in: SESSION_LIFETIME_SECONDS,
    }))
}

async fn hash_password(state: &AppState, password: String) -> Result<String, ApiError> {
    let permit = state
        .password_jobs
        .clone()
        .acquire_owned()
        .await
        .map_err(|error| {
            tracing::error!(%error, "no se pudo reservar capacidad para Argon2");
            ApiError::Internal
        })?;
    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let _permit = permit;
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow::anyhow!("no se pudo calcular el hash Argon2: {error}"))
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "falló el trabajo Argon2");
        ApiError::Internal
    })?
    .map_err(|error| {
        tracing::error!(%error, "falló el hash Argon2");
        ApiError::Internal
    })
}

async fn verify_password(
    state: &AppState,
    password_hash: String,
    password: String,
) -> Result<bool, ApiError> {
    let permit = state
        .password_jobs
        .clone()
        .acquire_owned()
        .await
        .map_err(|error| {
            tracing::error!(%error, "no se pudo reservar capacidad para Argon2");
            ApiError::Internal
        })?;
    tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
        let _permit = permit;
        let parsed_hash = PasswordHash::new(&password_hash)
            .map_err(|error| anyhow::anyhow!("hash Argon2 inválido: {error}"))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "falló el trabajo de verificación Argon2");
        ApiError::Internal
    })?
    .map_err(|error| {
        tracing::error!(%error, "falló la verificación Argon2");
        ApiError::Internal
    })
}

fn new_session_token() -> Result<(String, String), ApiError> {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| {
            tracing::error!(%error, "no se pudo generar un token de sesión");
            ApiError::Internal
        })?;
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    Ok((token, token_hash))
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty() && token.len() <= 128)
        .ok_or(ApiError::Unauthorized)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

async fn authenticate(headers: &HeaderMap, database: &PgPool) -> Result<i64, ApiError> {
    let token_hash = hash_token(bearer_token(headers)?);
    sqlx::query_scalar::<_, i64>(
        "SELECT player_id FROM sessions WHERE token_hash = $1 AND expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(database)
    .await
    .map_err(|error| database_error(error, "sesión inválida"))?
    .ok_or(ApiError::Unauthorized)
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode, ApiError> {
    let token_hash = hash_token(bearer_token(&headers)?);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(&state.database)
        .await
        .map_err(|error| database_error(error, "sesión inválida"))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateCreatureRequest {
    name: String,
    class: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct CreatureResponse {
    id: i64,
    name: String,
    class: String,
}

async fn create_creature(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateCreatureRequest>,
) -> Result<(StatusCode, Json<CreatureResponse>), ApiError> {
    let player_id = authenticate(&headers, &state.database).await?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 32 || name.chars().any(char::is_control) {
        return Err(ApiError::Validation(
            "el nombre debe tener entre 1 y 32 caracteres",
        ));
    }
    if !matches!(input.class.as_str(), "explorer" | "tank" | "hacker") {
        return Err(ApiError::Validation(
            "la clase debe ser explorer, tank o hacker",
        ));
    }

    let creature = sqlx::query_as::<_, CreatureResponse>(
        "INSERT INTO creatures (player_id, name, class) VALUES ($1, $2, $3) \
         RETURNING id, name, class",
    )
    .bind(player_id)
    .bind(name)
    .bind(input.class)
    .fetch_one(&state.database)
    .await
    .map_err(|error| database_error(error, "cada cuenta puede tener una criatura"))?;
    Ok((StatusCode::CREATED, Json(creature)))
}

#[derive(Deserialize)]
struct CreateExpeditionRequest {
    depth: i16,
}

#[derive(Serialize, sqlx::FromRow)]
struct ExpeditionResponse {
    id: i64,
    depth: i16,
    seed: i64,
    status: String,
    ends_at: String,
}

async fn create_expedition(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateExpeditionRequest>,
) -> Result<(StatusCode, Json<ExpeditionResponse>), ApiError> {
    let player_id = authenticate(&headers, &state.database).await?;
    if !(1..=3).contains(&input.depth) {
        return Err(ApiError::Validation(
            "la profundidad debe estar entre 1 y 3",
        ));
    }

    let creature_id = sqlx::query_scalar::<_, i64>("SELECT id FROM creatures WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(&state.database)
        .await
        .map_err(|error| database_error(error, "no se pudo consultar la criatura"))?
        .ok_or(ApiError::NotFound)?;

    let mut seed_bytes = [0_u8; 8];
    rand::rngs::OsRng
        .try_fill_bytes(&mut seed_bytes)
        .map_err(|error| {
            tracing::error!(%error, "no se pudo generar semilla de expedición");
            ApiError::Internal
        })?;
    let seed = i64::from_be_bytes(seed_bytes);

    let expedition = sqlx::query_as::<_, ExpeditionResponse>(
        "INSERT INTO expeditions \
            (player_id, creature_id, depth, seed, ends_at) \
         VALUES ($1, $2, $3, $4, now() + ($5::bigint * $3::bigint) * interval '1 second') \
         RETURNING id, depth, seed, status, ends_at::text AS ends_at",
    )
    .bind(player_id)
    .bind(creature_id)
    .bind(input.depth)
    .bind(seed)
    .bind(state.expedition_seconds_per_depth)
    .fetch_one(&state.database)
    .await
    .map_err(|error| database_error(error, "la criatura ya tiene una expedición activa"))?;

    Ok((StatusCode::ACCEPTED, Json(expedition)))
}

#[derive(Serialize, sqlx::FromRow)]
struct ExpeditionEventResponse {
    tick: i32,
    description: String,
    loot: serde_json::Value,
    created_at: String,
}

#[derive(Serialize)]
struct ExpeditionDetails {
    expedition: ExpeditionResponse,
    events: Vec<ExpeditionEventResponse>,
}

async fn get_expedition(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(expedition_id): Path<i64>,
) -> Result<Json<ExpeditionDetails>, ApiError> {
    let player_id = authenticate(&headers, &state.database).await?;
    let expedition = sqlx::query_as::<_, ExpeditionResponse>(
        "SELECT id, depth, seed, status, ends_at::text AS ends_at \
         FROM expeditions WHERE id = $1 AND player_id = $2",
    )
    .bind(expedition_id)
    .bind(player_id)
    .fetch_optional(&state.database)
    .await
    .map_err(|error| database_error(error, "no se pudo consultar la expedición"))?
    .ok_or(ApiError::NotFound)?;
    let events = sqlx::query_as::<_, ExpeditionEventResponse>(
        "SELECT tick, description, loot, created_at::text AS created_at \
         FROM expedition_events WHERE expedition_id = $1 ORDER BY tick",
    )
    .bind(expedition_id)
    .fetch_all(&state.database)
    .await
    .map_err(|error| database_error(error, "no se pudieron consultar los eventos"))?;

    Ok(Json(ExpeditionDetails { expedition, events }))
}

#[cfg(test)]
mod tests {
    use super::{validate_password, validate_username, RateLimiter};
    use std::{net::IpAddr, str::FromStr};

    #[test]
    fn accepts_only_normalized_usernames() {
        assert!(validate_username("noche_7"));
        assert!(!validate_username("Noche"));
        assert!(!validate_username("ab"));
        assert!(!validate_username("nombre con espacios"));
    }

    #[test]
    fn bounds_password_length() {
        assert!(!validate_password("demasiado"));
        assert!(validate_password("bateria correcta 123"));
        assert!(!validate_password(&"x".repeat(129)));
    }

    #[test]
    fn limits_auth_attempts_per_client() {
        let limiter = RateLimiter::default();
        let address = IpAddr::from_str("192.0.2.10").expect("IP fixture is valid");
        for _ in 0..10 {
            assert!(limiter.allow(address));
        }
        assert!(!limiter.allow(address));
    }
}
