use chrono::{Duration, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use zutility_be::db;

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("TEST_DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(24)
        .connect(&database_url)
        .await
        .ok()
}

struct Fixture {
    gift_id: Uuid,
    user_id: Uuid,
}

async fn fixture(pool: &PgPool) -> Option<Fixture> {
    sqlx::migrate!("./migrations").run(pool).await.ok()?;

    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, 'x')")
        .bind(user_id)
        .bind(format!("{user_id}@gift.test"))
        .execute(pool)
        .await
        .ok()?;

    let rate_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rate_snapshots (id, zec_ngn, zec_usd, usd_ngn, sources_used, fetched_at)
         VALUES ($1, 1000000, 50, 2000, $2, $3)",
    )
    .bind(rate_id)
    .bind(vec![String::from("test")])
    .bind(Utc::now())
    .execute(pool)
    .await
    .ok()?;

    let address = format!("ztestsapling1{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO deposit_addresses (address, address_type, network, used)
         VALUES ($1, 'shielded', 'testnet', true)",
    )
    .bind(&address)
    .execute(pool)
    .await
    .ok()?;

    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO orders (
            id, status, access_token_hash, utility_type, utility_slug, service_ref,
            amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
            required_confs, expires_at, user_id
         ) VALUES ($1, 'awaiting_payment', 'hash', 'airtime', 'mtn', '', 5000,
                   $2, 'shielded', 0.005, $3, 10, $4, $5)",
    )
    .bind(order_id)
    .bind(&address)
    .bind(rate_id)
    .bind(Utc::now() + Duration::minutes(30))
    .bind(user_id)
    .execute(pool)
    .await
    .ok()?;

    let gift_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gift_invites (
            id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
            amount_ngn, claim_window_hours, status, claim_expires_at
         ) VALUES ($1, $2, $3, $4, 'airtime', 'mtn', 5000, 24, 'open', $5)",
    )
    .bind(gift_id)
    .bind(format!("hash-{gift_id}"))
    .bind(user_id)
    .bind(order_id)
    .bind(Utc::now() + Duration::hours(24))
    .execute(pool)
    .await
    .ok()?;

    Some(Fixture { gift_id, user_id })
}

#[tokio::test]
async fn outstanding_sum_decodes_and_counts_open_gifts() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let Some(fixture) = fixture(&pool).await else {
        return;
    };

    let mut tx = pool.begin().await.expect("begin");
    let total = db::lock_user_and_sum_open_gifts(&mut tx, fixture.user_id)
        .await
        .expect("outstanding sum");
    tx.rollback().await.expect("rollback");

    assert_eq!(total, 5000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn only_one_concurrent_claim_wins() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let Some(fixture) = fixture(&pool).await else {
        return;
    };

    let mut handles = Vec::new();
    for index in 0..12 {
        let pool = pool.clone();
        let gift_id = fixture.gift_id;
        handles.push(tokio::spawn(async move {
            let mut tx = pool.begin().await.expect("begin");
            let won = db::claim_gift_tx(
                &mut tx,
                gift_id,
                &format!("08000000{index:03}"),
                Some("Recipient"),
            )
            .await
            .expect("claim query");

            if won {
                tx.commit().await.expect("commit");
            } else {
                tx.rollback().await.expect("rollback");
            }

            won
        }));
    }

    let mut wins = 0;
    for handle in handles {
        if handle.await.expect("join") {
            wins += 1;
        }
    }

    assert_eq!(wins, 1, "exactly one claim must win");

    let status: String = sqlx::query_scalar("SELECT status FROM gift_invites WHERE id = $1")
        .bind(fixture.gift_id)
        .fetch_one(&pool)
        .await
        .expect("gift status");
    assert_eq!(status, "claimed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claim_and_cancel_cannot_both_win() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let Some(fixture) = fixture(&pool).await else {
        return;
    };

    let claim_pool = pool.clone();
    let gift_id = fixture.gift_id;
    let claim = tokio::spawn(async move {
        let mut tx = claim_pool.begin().await.expect("begin");
        let won = db::claim_gift_tx(&mut tx, gift_id, "08000000001", None)
            .await
            .expect("claim query");

        if won {
            tx.commit().await.expect("commit");
        } else {
            tx.rollback().await.expect("rollback");
        }

        won
    });

    let cancel_pool = pool.clone();
    let cancel = tokio::spawn(async move {
        let mut tx = cancel_pool.begin().await.expect("begin");
        let won = db::cancel_gift_tx(&mut tx, gift_id)
            .await
            .expect("cancel query");

        if won {
            tx.commit().await.expect("commit");
        } else {
            tx.rollback().await.expect("rollback");
        }

        won
    });

    let claimed = claim.await.expect("join claim");
    let cancelled = cancel.await.expect("join cancel");

    assert_ne!(
        claimed, cancelled,
        "a gift cannot be both claimed and cancelled"
    );

    let status: String = sqlx::query_scalar("SELECT status FROM gift_invites WHERE id = $1")
        .bind(fixture.gift_id)
        .fetch_one(&pool)
        .await
        .expect("gift status");
    assert!(status == "claimed" || status == "cancelled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn claim_and_expiry_cannot_both_win() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let Some(fixture) = fixture(&pool).await else {
        return;
    };

    let claim_pool = pool.clone();
    let gift_id = fixture.gift_id;
    let claim = tokio::spawn(async move {
        let mut tx = claim_pool.begin().await.expect("begin");
        let won = db::claim_gift_tx(&mut tx, gift_id, "08000000000", None)
            .await
            .expect("claim query");

        if won {
            tx.commit().await.expect("commit");
        } else {
            tx.rollback().await.expect("rollback");
        }

        won
    });

    let expire_pool = pool.clone();
    let expire = tokio::spawn(async move {
        db::expire_gift_cas(&expire_pool, gift_id)
            .await
            .expect("expire query")
    });

    let claimed = claim.await.expect("join claim");
    let expired = expire.await.expect("join expire");

    assert_ne!(
        claimed, expired,
        "a gift cannot be both claimed and expired"
    );

    let status: String = sqlx::query_scalar("SELECT status FROM gift_invites WHERE id = $1")
        .bind(fixture.gift_id)
        .fetch_one(&pool)
        .await
        .expect("gift status");
    assert!(status == "claimed" || status == "expired");
}
