use axum::{
    body::{to_bytes, Body},
    extract::connect_info::ConnectInfo,
    http::{Request, Response, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

use abyssmarket_server::{api::router, expeditions::roll_reward, worker::resolve_due_expeditions};

#[sqlx::test(migrations = "./migrations")]
async fn account_can_create_and_reproduce_an_expedition(
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = router(pool.clone(), 60);
    let register_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            json!({"username": "test_user_1", "password": "correct horse battery staple"}),
            None,
            true,
        ))
        .await?;
    assert_eq!(register_response.status(), StatusCode::CREATED);
    let registration = response_json(register_response).await?;
    let player_id = registration["player"]["id"]
        .as_i64()
        .ok_or("missing player id")?;
    let token = registration["access_token"]
        .as_str()
        .ok_or("missing access token")?
        .to_owned();

    let password_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await?;
    assert!(password_hash.starts_with("$argon2id$"));
    assert!(!password_hash.contains("correct horse battery staple"));

    let login_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/login",
            json!({"username": "test_user_1", "password": "correct horse battery staple"}),
            None,
            true,
        ))
        .await?;
    assert_eq!(login_response.status(), StatusCode::OK);

    let creature_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/creatures",
            json!({"name": "Polilla", "class": "explorer"}),
            Some(&token),
            false,
        ))
        .await?;
    assert_eq!(creature_response.status(), StatusCode::CREATED);

    let invalid_depth = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/expeditions",
            json!({"depth": 4}),
            Some(&token),
            false,
        ))
        .await?;
    assert_eq!(invalid_depth.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expedition_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/expeditions",
            json!({"depth": 1}),
            Some(&token),
            false,
        ))
        .await?;
    assert_eq!(expedition_response.status(), StatusCode::ACCEPTED);
    let expedition_json = response_json(expedition_response).await?;
    let expedition_id = expedition_json["id"]
        .as_i64()
        .ok_or("missing expedition id")?;
    let seed = expedition_json["seed"]
        .as_i64()
        .ok_or("missing expedition seed")?;

    let second_expedition = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/expeditions",
            json!({"depth": 2}),
            Some(&token),
            false,
        ))
        .await?;
    assert_eq!(second_expedition.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE expeditions SET ends_at = now() - interval '1 second' WHERE id = $1")
        .bind(expedition_id)
        .execute(&pool)
        .await?;

    assert_eq!(resolve_due_expeditions(&pool).await?, 1);
    assert_eq!(resolve_due_expeditions(&pool).await?, 0);

    let details_response = app
        .oneshot(json_request(
            "GET",
            &format!("/api/expeditions/{expedition_id}"),
            Value::Null,
            Some(&token),
            false,
        ))
        .await?;
    assert_eq!(details_response.status(), StatusCode::OK);
    let details = response_json(details_response).await?;
    assert_eq!(details["expedition"]["status"], "completed");

    let rules = sqlx::query_as::<_, (i64, String, String, i32, i32, i32)>(
        "SELECT i.id, i.slug, i.name, r.min_quantity, r.max_quantity, r.weight \
         FROM expedition_rewards r JOIN items i ON i.id = r.item_id WHERE r.depth = 1",
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(
        |(item_id, item_slug, item_name, min_quantity, max_quantity, weight)| {
            abyssmarket_server::expeditions::RewardRule {
                item_id,
                item_slug,
                item_name,
                min_quantity,
                max_quantity,
                weight,
            }
        },
    )
    .collect::<Vec<_>>();
    let expected_reward = roll_reward(seed, &rules).ok_or("no reward rule for depth 1")?;
    assert_eq!(
        details["events"][0]["loot"][0]["item"],
        expected_reward.item_slug
    );
    assert_eq!(
        details["events"][0]["loot"][0]["quantity"],
        expected_reward.quantity
    );

    let inventory_quantity: i64 =
        sqlx::query_scalar("SELECT quantity FROM inventory WHERE player_id = $1 AND item_id = $2")
            .bind(player_id)
            .bind(expected_reward.item_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(inventory_quantity, i64::from(expected_reward.quantity));

    Ok(())
}

fn json_request(
    method: &str,
    uri: &str,
    body: Value,
    token: Option<&str>,
    include_connect_info: bool,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let mut request = builder
        .body(Body::from(body.to_string()))
        .expect("request fixture is valid");
    if include_connect_info {
        request.extensions_mut().insert(ConnectInfo(
            "192.0.2.20:5000"
                .parse::<std::net::SocketAddr>()
                .expect("socket address fixture is valid"),
        ));
    }
    request
}

async fn response_json(response: Response<Body>) -> Result<Value, Box<dyn std::error::Error>> {
    let bytes = to_bytes(response.into_body(), 16 * 1024).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
