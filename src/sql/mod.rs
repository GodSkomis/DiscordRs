
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
