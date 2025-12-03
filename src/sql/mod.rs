pub mod autoroom;


pub mod prelude {
    use serenity::prelude::TypeMapKey;
    use sqlx::PgPool;

    pub use super::autoroom::{AutoRoom, MonitoredAutoRoom};
    use super::SerenityPool;
    
    impl TypeMapKey for SerenityPool {
        type Value = PgPool;
    }

    pub async fn create_tables(pool : &PgPool) {
        AutoRoom::create_table(pool).await;
        MonitoredAutoRoom::create_table(pool).await;
    }
}

pub struct SerenityPool;


pub mod pool {
    use once_cell::sync::OnceCell;
    use sqlx::{Pool, Postgres};

    type PoolType = Pool<Postgres>;

    pub struct SqlPool {
        pool: PoolType
    }

    impl SqlPool {
        pub fn get_pool(&self) -> PoolType {
            self.pool.clone()
        }

        pub fn new(pool: PoolType) -> Self {
            Self { pool }
        }
    }

    pub static GLOBAL_SQL_POOL: OnceCell<SqlPool> = OnceCell::new();
}