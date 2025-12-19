use serenity::{all::VoiceState, async_trait};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod voice;
mod bitrate;
mod sql;
mod commands;
pub mod services;

use voice::{create_proccessing, remove_channel_by_voicestate, VoiceProccessing};
use sql::{prelude::*, SerenityPool};

use crate::services::autoroom::cleanup_categories_monitored_rooms;
use crate::sql::pool::SqlPool;
use crate::{services::autoroom::cleanup_db_monitored_rooms, sql::pool::GLOBAL_SQL_POOL};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("`{}` is now online", ready.user.name);
        let mut err = cleanup_db_monitored_rooms(&ctx).await.err();
        if err.is_some() {
            tracing::error!(err);
        };
        err = cleanup_categories_monitored_rooms(&ctx).await.err();
        if err.is_some() {
            tracing::error!(err);
        };
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(response) = VoiceProccessing.proccess(&ctx, &msg).await {
            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                tracing::error!("Error sending message: {why:?}");
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        create_proccessing(&ctx, &new).await;
        if let Some(voice_state) = old {
            let err = match remove_channel_by_voicestate(&ctx, &voice_state).await {
                Ok(_) => return,
                Err(_err) => _err,
            };
            tracing::error!(err);
        };
    }
}

#[tokio::main]
async fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN").unwrap();

    let db_url = std::env::var("POSTGRES_URI").unwrap();
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url).await.unwrap();

    // tracing::info!("Drop table has begun");
    // sqlx::query("DROP table monitored_autoroom").execute(&db).await.expect("Drop table 'monitored_autoroom' unsuccessful");
    // tracing::info!("Drop table has been completed");

    tracing::info!("Table creation has begun");
    if let Err(err) = create_tables(&db).await {
        tracing::error!("Error while creationg sql tables. Finishing...\n Error: `{}`", err);
        return;
    };
    tracing::info!("Table creation has been completed");

    match GLOBAL_SQL_POOL.set(SqlPool::new(db.clone())) {
        Ok(_) => (),
        Err(_) => {
            tracing::error!("GLOBAL_SQL_POOL isn't empty");
            GLOBAL_SQL_POOL.get().expect("Unable to get GLOBAL_SQL_POOL");
        },
    };


    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(commands::generate_commands_framework(db.clone()).await)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SerenityPool>(db.clone());
    };

    tracing::info!("Starting discord bot");
    if let Err(why) = client.start().await {
        tracing::info!("Client error: {:?}", why);
    };
    tracing::info!("Finishing discord bot");

}