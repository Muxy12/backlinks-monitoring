//! Repository + tenant-scope integration tests. Need real Postgres; gated on
//! `DATABASE_URL` (skip-and-pass when unset). CI's rust/migrations jobs set it.
//!
//! Each test uses fresh random emails/slugs so parallel runs against the shared
//! CI database do not collide.

use auth::{Argon2Params, hash_password};
use common::Role;
use db::repositories::{orgs, sessions, users};
use db::{DbConfig, DbError, authorize_org, build_pool, migrate};
use uuid::Uuid;

async fn setup() -> Option<db::Db> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())?;
    let db = build_pool(&DbConfig::worker(&url, 4, None))
        .await
        .expect("pool");
    migrate::run(&db.pool).await.expect("migrate");
    Some(db)
}

fn uniq(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}

#[tokio::test]
async fn user_create_find_and_unique_email() {
    let Some(db) = setup().await else { return };
    let email = format!("{}@example.com", uniq("u"));
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();

    let created = users::create(&db.pool, Uuid::now_v7(), &email, &hash, "Alice")
        .await
        .expect("create user");
    assert_eq!(created.email, email);
    assert!(!created.is_email_verified());

    let found = users::find_by_email(&db.pool, &email)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, created.id);

    // Duplicate email -> Conflict, not a 500.
    let dup = users::create(&db.pool, Uuid::now_v7(), &email, &hash, "Bob").await;
    assert!(
        matches!(dup, Err(DbError::Conflict(_))),
        "duplicate email must conflict"
    );
}

#[tokio::test]
async fn create_org_makes_creator_the_owner() {
    let Some(db) = setup().await else { return };
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();
    let user = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("o")),
        &hash,
        "Owner",
    )
    .await
    .unwrap();

    let org = orgs::create_with_owner(&db.pool, Uuid::now_v7(), "Acme", &uniq("acme"), user.id)
        .await
        .expect("create org");

    // The creator can authorize into the org as owner.
    let scope = authorize_org(&db.pool, user.id, org.id)
        .await
        .expect("owner is a member");
    assert_eq!(scope.role(), Role::Owner);
    assert!(scope.require(Role::Admin).is_ok(), "owner satisfies admin");
    assert!(scope.require(Role::Owner).is_ok());
}

#[tokio::test]
async fn non_member_cannot_authorize_and_gets_not_found() {
    let Some(db) = setup().await else { return };
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();
    let owner = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("owner")),
        &hash,
        "O",
    )
    .await
    .unwrap();
    let outsider = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("out")),
        &hash,
        "X",
    )
    .await
    .unwrap();
    let org = orgs::create_with_owner(&db.pool, Uuid::now_v7(), "Acme", &uniq("acme"), owner.id)
        .await
        .unwrap();

    // The IDOR guarantee: a non-member gets NotFound, never Forbidden (docs/05 §3).
    match authorize_org(&db.pool, outsider.id, org.id).await {
        Err(DbError::NotFound) => {}
        other => panic!("non-member must get NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn list_for_user_returns_orgs_with_roles() {
    let Some(db) = setup().await else { return };
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();
    let user = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("m")),
        &hash,
        "M",
    )
    .await
    .unwrap();
    orgs::create_with_owner(&db.pool, Uuid::now_v7(), "One", &uniq("one"), user.id)
        .await
        .unwrap();
    orgs::create_with_owner(&db.pool, Uuid::now_v7(), "Two", &uniq("two"), user.id)
        .await
        .unwrap();

    let orgs = orgs::list_for_user(&db.pool, user.id).await.unwrap();
    assert!(orgs.len() >= 2);
    assert!(orgs.iter().all(|(_, role)| *role == Role::Owner));
}

#[tokio::test]
async fn session_lifecycle_active_then_revoked() {
    let Some(db) = setup().await else { return };
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();
    let user = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("s")),
        &hash,
        "S",
    )
    .await
    .unwrap();

    let token = auth::token::Token::generate();
    let csrf = auth::token::Token::generate();
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(1);

    let sess = sessions::create(
        &db.pool,
        Uuid::now_v7(),
        user.id,
        &token.hash,
        &csrf.hash,
        expires,
    )
    .await
    .expect("create session");

    // Found while active.
    let found = sessions::find_active_by_token_hash(&db.pool, &token.hash, now)
        .await
        .unwrap();
    assert_eq!(found.id, sess.id);
    assert!(found.is_active(now));

    // Revoked -> no longer findable.
    sessions::revoke(&db.pool, sess.id).await.unwrap();
    assert!(matches!(
        sessions::find_active_by_token_hash(&db.pool, &token.hash, now).await,
        Err(DbError::NotFound)
    ));
}

#[tokio::test]
async fn expired_session_is_not_active() {
    let Some(db) = setup().await else { return };
    let hash = hash_password("pw", &Argon2Params::insecure_fast()).unwrap();
    let user = users::create(
        &db.pool,
        Uuid::now_v7(),
        &format!("{}@x.com", uniq("e")),
        &hash,
        "E",
    )
    .await
    .unwrap();
    let token = auth::token::Token::generate();
    let csrf = auth::token::Token::generate();
    let now = chrono::Utc::now();

    // Already expired.
    sessions::create(
        &db.pool,
        Uuid::now_v7(),
        user.id,
        &token.hash,
        &csrf.hash,
        now - chrono::Duration::hours(1),
    )
    .await
    .unwrap();
    assert!(matches!(
        sessions::find_active_by_token_hash(&db.pool, &token.hash, now).await,
        Err(DbError::NotFound)
    ));
}
