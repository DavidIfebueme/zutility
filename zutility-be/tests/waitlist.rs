use chrono::{Duration, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use zutility_be::db::{get_waitlist_position, latest_waitlist_token_age_seconds};

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("TEST_DATABASE_URL").ok()?;
    Some(
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("TEST_DATABASE_URL is set but the test database is unreachable"),
    )
}

async fn prepare_schema(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("test database migrations failed");
    let _ = sqlx::query("TRUNCATE TABLE waitlist_verify_tokens, waitlist_entries CASCADE")
        .execute(pool)
        .await;
}

async fn insert_entry(
    pool: &PgPool,
    email: &str,
    code: &str,
    verified: bool,
    referred_by: Option<&str>,
    created_at: chrono::DateTime<Utc>,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO waitlist_entries (email, referral_code, email_verified, referred_by, created_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(email)
    .bind(code)
    .bind(verified)
    .bind(referred_by)
    .bind(created_at)
    .fetch_one(pool)
    .await
    .expect("insert waitlist entry")
}

#[tokio::test]
async fn position_counts_verified_entries_and_rewards_referrals() {
    let Some(pool) = test_pool().await else {
        return;
    };
    prepare_schema(&pool).await;

    let base = Utc::now() - Duration::hours(1);
    let a = insert_entry(&pool, "a@example.com", "AAAA1111", true, None, base).await;
    let b = insert_entry(
        &pool,
        "b@example.com",
        "BBBB2222",
        true,
        None,
        base + Duration::minutes(1),
    )
    .await;
    let c = insert_entry(
        &pool,
        "c@example.com",
        "CCCC3333",
        false,
        None,
        base + Duration::minutes(2),
    )
    .await;
    let d = insert_entry(
        &pool,
        "d@example.com",
        "DDDD4444",
        true,
        Some("BBBB2222"),
        base + Duration::minutes(3),
    )
    .await;

    assert_eq!(get_waitlist_position(&pool, a).await.expect("a"), 1);
    assert_eq!(get_waitlist_position(&pool, b).await.expect("b"), 1);
    assert_eq!(get_waitlist_position(&pool, d).await.expect("d"), 3);
    assert_eq!(get_waitlist_position(&pool, c).await.expect("c"), 3);

    let e = insert_entry(
        &pool,
        "e@example.com",
        "EEEE5555",
        true,
        Some("BBBB2222"),
        base + Duration::minutes(4),
    )
    .await;

    assert_eq!(get_waitlist_position(&pool, b).await.expect("b"), 1);
    assert_eq!(get_waitlist_position(&pool, e).await.expect("e"), 4);

    sqlx::query("UPDATE waitlist_entries SET email_verified = true WHERE id = $1")
        .bind(c)
        .execute(&pool)
        .await
        .expect("verify c");
    assert_eq!(get_waitlist_position(&pool, c).await.expect("c"), 3);
    assert_eq!(get_waitlist_position(&pool, d).await.expect("d"), 4);
}

#[tokio::test]
async fn token_age_supports_the_resend_cooldown() {
    let Some(pool) = test_pool().await else {
        return;
    };
    prepare_schema(&pool).await;

    let entry = insert_entry(
        &pool,
        "age@example.com",
        "AGE00001",
        false,
        None,
        Utc::now(),
    )
    .await;

    assert_eq!(
        latest_waitlist_token_age_seconds(&pool, entry)
            .await
            .expect("no token"),
        None
    );

    sqlx::query(
        "INSERT INTO waitlist_verify_tokens (entry_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, now() + interval '1 hour', now())",
    )
    .bind(entry)
    .bind("hash")
    .execute(&pool)
    .await
    .expect("insert token");

    let fresh = latest_waitlist_token_age_seconds(&pool, entry)
        .await
        .expect("fresh token")
        .expect("age");
    assert!(fresh < 60);

    sqlx::query("UPDATE waitlist_verify_tokens SET created_at = now() - interval '2 minutes' WHERE entry_id = $1")
        .bind(entry)
        .execute(&pool)
        .await
        .expect("age token");

    let old = latest_waitlist_token_age_seconds(&pool, entry)
        .await
        .expect("old token")
        .expect("age");
    assert!(old >= 60);
}
