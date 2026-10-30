use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use tokio::time;

use crate::expeditions::{roll_reward, RewardRule};

const MAX_BATCH_SIZE: i64 = 25;

#[derive(FromRow)]
struct DueExpedition {
    id: i64,
    player_id: i64,
    depth: i16,
    seed: i64,
}

#[derive(FromRow)]
struct RewardRow {
    item_id: i64,
    item_slug: String,
    item_name: String,
    min_quantity: i32,
    max_quantity: i32,
    weight: i32,
}

impl From<RewardRow> for RewardRule {
    fn from(row: RewardRow) -> Self {
        Self {
            item_id: row.item_id,
            item_slug: row.item_slug,
            item_name: row.item_name,
            min_quantity: row.min_quantity,
            max_quantity: row.max_quantity,
            weight: row.weight,
        }
    }
}

pub async fn resolve_due_expeditions(database: &PgPool) -> Result<u64> {
    let mut transaction = database.begin().await?;
    let expeditions = sqlx::query_as::<_, DueExpedition>(
        "SELECT id, player_id, depth, seed FROM expeditions \
         WHERE status = 'running' AND ends_at <= now() \
         ORDER BY ends_at LIMIT $1 FOR UPDATE SKIP LOCKED",
    )
    .bind(MAX_BATCH_SIZE)
    .fetch_all(&mut *transaction)
    .await?;

    for expedition in &expeditions {
        let rules = sqlx::query_as::<_, RewardRow>(
            "SELECT i.id AS item_id, i.slug AS item_slug, i.name AS item_name, \
                    r.min_quantity, r.max_quantity, r.weight \
             FROM expedition_rewards r JOIN items i ON i.id = r.item_id \
             WHERE r.depth = $1 ORDER BY r.id",
        )
        .bind(expedition.depth)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(RewardRule::from)
        .collect::<Vec<_>>();
        let reward = roll_reward(expedition.seed, &rules).with_context(|| {
            format!(
                "no hay reglas válidas para profundidad {}",
                expedition.depth
            )
        })?;

        sqlx::query(
            "INSERT INTO inventory (player_id, item_id, quantity) VALUES ($1, $2, $3) \
             ON CONFLICT (player_id, item_id) DO UPDATE \
             SET quantity = inventory.quantity + EXCLUDED.quantity",
        )
        .bind(expedition.player_id)
        .bind(reward.item_id)
        .bind(i64::from(reward.quantity))
        .execute(&mut *transaction)
        .await
        .with_context(|| {
            format!(
                "no se pudo otorgar el botín de expedición {}",
                expedition.id
            )
        })?;

        let loot = json!([{
            "item": reward.item_slug,
            "quantity": reward.quantity
        }]);
        let description = format!(
            "Tu criatura regresó con {} {}.",
            reward.quantity, reward.item_name
        );
        sqlx::query(
            "INSERT INTO expedition_events (expedition_id, tick, description, loot) \
             VALUES ($1, 1, $2, $3)",
        )
        .bind(expedition.id)
        .bind(description)
        .bind(loot)
        .execute(&mut *transaction)
        .await
        .with_context(|| {
            format!(
                "no se pudo guardar el diario de expedición {}",
                expedition.id
            )
        })?;

        sqlx::query(
            "UPDATE expeditions SET status = 'completed', completed_at = now() \
             WHERE id = $1 AND status = 'running'",
        )
        .bind(expedition.id)
        .execute(&mut *transaction)
        .await
        .with_context(|| format!("no se pudo completar la expedición {}", expedition.id))?;
    }

    transaction.commit().await?;
    Ok(expeditions.len() as u64)
}

pub async fn run(database: PgPool) -> Result<()> {
    let mut poll_interval = time::interval(Duration::from_secs(1));
    let shutdown_signal = tokio::signal::ctrl_c();
    tokio::pin!(shutdown_signal);
    tracing::info!("worker de expediciones iniciado");

    loop {
        tokio::select! {
            signal = &mut shutdown_signal => {
                signal.context("no se pudo escuchar la señal de apagado")?;
                tracing::info!("worker de expediciones detenido");
                return Ok(());
            }
            _ = poll_interval.tick() => {
                match resolve_due_expeditions(&database).await {
                    Ok(resolved) if resolved > 0 => {
                        tracing::info!(resolved, "expediciones resueltas");
                    }
                    Ok(_) => {}
                    Err(error) => tracing::error!(%error, "falló la resolución de expediciones"),
                }
            }
        }
    }
}
