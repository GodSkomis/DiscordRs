pub mod autoroom;


pub mod prelude {
    use serenity::prelude::TypeMapKey;
    use sqlx::{Error, PgPool};

    use crate::sql::autoroom::table_builder::CreateTable;

    pub use super::autoroom::{AutoRoom, MonitoredAutoRoom};
    use super::SerenityPool;
    
    impl TypeMapKey for SerenityPool {
        type Value = PgPool;
    }

    pub async fn create_tables(pool : &PgPool) -> Result<(), Error> {
        AutoRoom::create_table(pool).await?;
        MonitoredAutoRoom::create_table(pool).await?;

        Ok(())
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