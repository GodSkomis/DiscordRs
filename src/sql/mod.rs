pub mod autoroom;
pub mod savedroom;


pub mod prelude {
    use serenity::prelude::TypeMapKey;
    use sqlx::PgPool;

    use crate::sql::savedroom::{SavedRoom, SavedRoomGuest};

    pub use super::autoroom::{AutoRoom, MonitoredAutoRoom};
    use super::SerenityPool;
    
    impl TypeMapKey for SerenityPool {
        type Value = PgPool;
    }

    pub async fn create_tables(pool : &PgPool) {
        AutoRoom::create_table(pool).await;
        MonitoredAutoRoom::create_table(pool).await;
        SavedRoom::create_table(pool).await;
        SavedRoomGuest::create_table(pool).await;
    }
}


pub struct SerenityPool;
