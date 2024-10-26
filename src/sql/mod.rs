
pub mod autoroom;

pub mod prelude {
    use serenity::prelude::TypeMapKey;
    use sqlx::{Pool, Sqlite, SqlitePool};

    pub use super::{
        DbPool,
        autoroom::{AutoRoom, MonitoredAutoRoom},
    };
    

    impl TypeMapKey for DbPool {
        type Value = SqlitePool;
    }

    pub async fn create_tables(pool : &Pool<Sqlite>) {
        AutoRoom::create_table(pool).await;
        MonitoredAutoRoom::create_table(pool).await;
    }
}

pub struct DbPool;
