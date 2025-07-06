use sqlx::{Error, FromRow, PgPool, Row};


#[derive(Debug, FromRow)]
pub struct AutoRoom {
    id: i32,
    pub channel_id: i64,
    pub category_id: i64,
    pub suffix: String
}

#[derive(Debug, )]
pub struct AutoRoomDTO {
    pub channel_id: i64,
    pub category_id: i64,
    pub suffix: String
}

#[derive(Debug, FromRow)]
pub struct MonitoredAutoRoom {
    pub channel_id: i64,
    pub owner_id: i64
}

impl AutoRoom {
    pub async fn get_by_channel_id(pool: &PgPool, channel_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, AutoRoom>("SELECT * from autoroom WHERE channel_id = $1")
            .bind(channel_id)
            .fetch_one(pool)
            .await {
            Ok(autoroom) => Ok(Some(autoroom)),
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }

    pub async fn get_by_category_id(pool: &PgPool, category_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, AutoRoom>("SELECT * from autoroom WHERE category_id = $1")
            .bind(category_id)
            .fetch_one(pool)
            .await {
            Ok(autoroom) => Ok(Some(autoroom)),
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }

    pub async fn insert(pool: &PgPool, dto: &AutoRoomDTO) {
        let query = "INSERT INTO autoroom (channel_id, category_id, suffix) VALUES ($1, $2, $3)";
        sqlx::query(query)
            .bind(dto.channel_id)
            .bind(dto.category_id)
            .bind(dto.suffix.clone())
            .execute(pool)
            .await
            .expect(
                &format!(
                    "Failed to insert AutoRoom, CHANNEL({}) CATEGORY({}) SUFFIX({})",
                    dto.channel_id,
                    dto.category_id,
                    dto.suffix
                )
            );
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}

impl MonitoredAutoRoom {
    pub async fn exists(pool: &PgPool, channel_id: i64) -> bool {
        let query = "SELECT EXISTS(SELECT 1 FROM monitored_autoroom WHERE channel_id = $1)";
        let result = sqlx::query(query)
            .bind(channel_id)
            .fetch_one(pool)
            .await;

        // Извлекаем значение из результата
        match result {
            Ok(row) => {
                let exists: bool = row.get(0); // Извлекаем значение
                exists
            },
            Err(err) => {
                println!("{}", err);
                false // В случае ошибки возвращаем false
            }
        }
    }

    pub async fn remove(pool: &PgPool, channel_id: i64) -> Result<bool, sqlx::Error> {
        let query = "DELETE FROM monitored_autoroom WHERE channel_id = $1";
        let result = sqlx::query(query)
        .bind(channel_id)
        .execute(pool)
        .await?;

        // Проверяем, были ли затронуты строки
        Ok(result.rows_affected() > 0)
    }
    
    pub async fn new(pool: &PgPool, channel_id: i64, owner_id: i64) {
        let query = "INSERT INTO monitored_autoroom (channel_id, owner_id) VALUES ($1, $2)";
        sqlx::query(query)
            .bind(channel_id)
            .bind(owner_id)
            .execute(pool)
            .await
            .expect(
                &format!(
                    "Failed to insert MonitoredAutoRoom, CHANNEL({}) OWNER({})",
                    channel_id,
                    owner_id
                )
            );
    }

    pub async fn get_by_owner_id(pool: &PgPool, owner_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, Self>("SELECT channel_id, owner_id from monitored_autoroom WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(pool)
            .await {
            Ok(monitored_autoroom) => Ok(Some(monitored_autoroom)),
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }
}

mod table_builder {
    use sqlx::PgPool;
    use super::{AutoRoom, MonitoredAutoRoom};

    impl AutoRoom {
        pub async fn create_table(pool : &PgPool) {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS autoroom (
                        id SERIAL PRIMARY KEY,
                        channel_id BIGINT UNIQUE NOT NULL,
                        category_id BIGINT NOT NULL,
                        suffix VARCHAR(16) NOT NULL
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("Failed to create autoroom table");
        }
    }
    impl MonitoredAutoRoom {
        pub async fn create_table(pool : &PgPool) {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS monitored_autoroom (
                        channel_id BIGINT PRIMARY KEY,
                        owner_id BIGINT NOT NULL
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("Failed to create autoroom table");
        }
    }
}