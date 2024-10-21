use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::voice::VoiceState;
use serenity::prelude::GatewayIntents;
use serenity::model::channel::Message;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqlitePool;
use dotenv::dotenv;
use sqlx::Sqlite;
use std::env;

mod voice;
mod bitrate;
mod sql;

use voice::{create_proccessing, remove_proccessing, VoiceProccessing};
use sql::{create_tables, DbPool};


struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(response) = VoiceProccessing.proccess(&ctx, &msg).await {
            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        create_proccessing(&ctx, &new).await;
        if let Some(voice_state) = old {
            remove_proccessing(&ctx, &voice_state).await;
        };
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let db_path = env::var("DISCORD_DB").expect("Expected a db path in the environment");
    if !Sqlite::database_exists(&db_path).await.unwrap_or(false) {
        println!("Creating database {}", &db_path);
        match Sqlite::create_database(&db_path).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    let pool = SqlitePool::connect(&db_path).await.expect("Failed to connect to the database");
    create_tables(&pool).await;

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

        {
            let mut data = client.data.write().await;
            data.insert::<DbPool>(pool);
        }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}