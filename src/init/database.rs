use crate::init::env::DATABASE_URL;
use once_cell::sync::OnceCell;
use sqlx::PgPool;

static POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn init() {
    POOL.set(PgPool::connect(&DATABASE_URL).await.unwrap())
        .unwrap();
}

pub fn get_pool() -> &'static PgPool {
    POOL.get().unwrap()
}
