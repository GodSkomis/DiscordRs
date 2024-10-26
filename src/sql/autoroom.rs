use sqlx::{Error, FromRow, Pool, Row, Sqlite, SqlitePool};


#[derive(Debug, FromRow)]
pub struct AutoRoom {
    pub channel_id: u64,
    pub category_id: u64,
    pub suffix: String
}

#[derive(Debug, FromRow)]
pub struct MonitoredAutoRoom {
    pub channel_id: u64,
}

impl AutoRoom {
    // Метод для получения пользователя по ID
    pub async fn get_by_channel_id(pool: &SqlitePool, channel_id: i64) -> Result<Option<Self>, Error> {
        match sqlx::query_as::<_, AutoRoom>("SELECT channel_id, category_id, suffix from autoroom WHERE channel_id = ?")
            .bind(channel_id)
            .fetch_one(pool)
            .await {
            Ok(autoroom) => Ok(Some(autoroom)), // Если пользователь найден, возвращаем его обёрнутым в Some
            Err(err) => match err {
                sqlx::Error::RowNotFound => Ok(None), // Если пользователь не найден, возвращаем None
                _ => Err(err), // Для других ошибок возвращаем их
            },
        }
    }

    pub async fn create(&self, pool: &Pool<Sqlite>) {
        let query = "INSERT INTO autoroom (channel_id, category_id, suffix) VALUES (?, ?, ?)";
        sqlx::query(query)
            .bind(self.channel_id as i64)
            .bind(self.category_id as i64)
            .bind(self.suffix.clone())
            .execute(pool)
            .await
            .expect("Failed to insert monitored autoroom");
    }
}

impl MonitoredAutoRoom {
    pub async fn exists(pool: &Pool<Sqlite>, channel_id: i64) -> bool {
        let query = "SELECT EXISTS(SELECT 1 FROM monitored_autoroom WHERE channel_id = ?)";
        let result = sqlx::query(query)
            .bind(channel_id)
            .fetch_one(pool)
            .await;

        // Извлекаем значение из результата
        match result {
            Ok(row) => {
                let exists: i32 = row.get(0); // Извлекаем значение
                exists == 1 // Возвращаем true или false
            },
            Err(err) => {
                println!("{}", err);
                false // В случае ошибки возвращаем false
            }
        }
    }

    pub async fn remove(&self, pool: &Pool<Sqlite>) -> Result<bool, sqlx::Error> {
        let query = "DELETE FROM monitored_autoroom WHERE channel_id = ?";
        let result = sqlx::query(query)
        .bind(self.channel_id as i64)
        .execute(pool)
        .await?;

        // Проверяем, были ли затронуты строки
        Ok(result.rows_affected() > 0)
    }
    
    pub async fn new(pool: &Pool<Sqlite>, channel_id: i64) {
        let query = "INSERT INTO monitored_autoroom (channel_id) VALUES (?)";
        sqlx::query(query)
            .bind(channel_id)
            .execute(pool)
            .await
            .expect("Failed to insert monitored autoroom");
    }
}

mod table_builder {
    use sqlx::{Pool, Sqlite};
    use super::{AutoRoom, MonitoredAutoRoom};

    impl AutoRoom {
        pub async fn create_table(pool : &Pool<Sqlite>) {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS autoroom (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
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
        pub async fn create_table(pool : &Pool<Sqlite>) {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS monitored_autoroom (
                        channel_id BIGINT PRIMARY KEY
                )
                "#,
            )
            .execute(pool)
            .await
            .expect("Failed to create autoroom table");
        }
    }
}