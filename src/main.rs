use dashmap::DashMap;
use once_cell::sync::OnceCell;
use serenity::all::{ChannelId, ComponentInteractionDataKind, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction, InviteCreateEvent, UserId};
use serenity::{all::VoiceState, async_trait};
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, prelude::*};
use tracing_better_stack::{BetterStackLayer, BetterStackConfig};

mod voice;
mod bitrate;
mod sql;
mod commands;
pub mod services;

use voice::{create_proccessing, remove_channel_by_voicestate};
use sql::{prelude::*, SerenityPool};

use crate::services::autoroom::cleanup_categories_monitored_rooms;
use crate::services::autoroom::voice_channel::invite_user;
use crate::sql::pool::SqlPool;
use crate::{services::autoroom::cleanup_db_monitored_rooms, sql::pool::GLOBAL_SQL_POOL};

struct Handler;


static SELECTED_USER_STORE: OnceCell<DashMap<UserId, UserId>> = OnceCell::new();


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

    async fn invite_create(&self, ctx: Context, data: InviteCreateEvent) {
        if let Some(guild_id) = data.guild_id {
            if let (Some(author), Some(user)) = (data.inviter, data.target_user) {
                let pool = &GLOBAL_SQL_POOL.get().unwrap().get_pool();
                match invite_user(ctx.http(), pool, author.id.get() as i64, &user).await {
                    Ok(_) => tracing::info!(
                        "Invite event, permissions gived.\nGUILD({}) INVITER({}) TARGET({})",
                        guild_id,
                        author,
                        user
                    ),
                    Err(err) => tracing::error!(
                        "Invite event, give permissions error.\nGUILD({}) INVITER({}) TARGET({})\n{}",
                        guild_id,
                        author,
                        user,
                        err
                    ),
                };
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // Нас интересуют только взаимодействия с компонентами (кнопки/меню)
        if let Interaction::Component(mci) = interaction {
            let custom_id = &mci.data.custom_id;

            // 1. Проверяем префикс через starts_with
            if custom_id.starts_with("inv_") {
                
                // Разделяем ID на части. 
                // Ожидаемый формат: "inv_тип_ктоСоздал_какойКанал"
                let parts: Vec<&str> = custom_id.split('_').collect();
                
                // Извлекаем тип действия (второй элемент после "inv")
                let action_type = parts.get(1).copied().unwrap_or("");
                
                // Безопасно парсим ID пользователя и канала
                let owner_id = parts.get(2).and_then(|s| s.parse::<u64>().ok()).map(UserId::new);
                let channel_id = parts.get(3).and_then(|s| s.parse::<u64>().ok()).map(ChannelId::new);

                match action_type {
                    "inv_sel" => {
                            if let (Some(owner), Some(channel)) = (owner_id, channel_id) {
                            
                                if mci.user.id != owner {
                                    let _ = mci.create_response(&ctx.http, CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new().content("You aren't host of the room").ephemeral(true)
                                    )).await;
                                    return;
                                }
                            
                                if let ComponentInteractionDataKind::UserSelect { values } = &mci.data.kind {
                                    if let Some(target_id) = values.first() {

                                        SELECTED_USER_STORE.get().unwrap().insert(owner, *target_id);

                                        if let Err(err) = mci.create_response(&ctx.http, CreateInteractionResponse::Acknowledge).await {
                                            tracing::error!("{:?}", err);
                                            return;
                                        };
                                    }
                                    
                                    if let Err(err) = mci.create_response(&ctx.http, CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new().content("Choose a member").ephemeral(true)
                                    )).await {
                                        tracing::error!("{:?}", err);
                                        return;
                                    };
                                }

                        }
                    }
                    "inv_inv" => {
                        if let (Some(owner), Some(channel)) = (owner_id, channel_id) {
                            if let Some(target_id) = SELECTED_USER_STORE.get().unwrap().get(&owner) {
                                if let Err(err) = mci.create_response(&ctx.http, CreateInteractionResponse::Acknowledge).await {
                                            tracing::error!("{:?}", err);
                                            return;
                                };

                                let target_id = *target_id;
                                let pool = GLOBAL_SQL_POOL.get().unwrap().get_pool();

                                let invited_user = match target_id.to_user(&ctx).await {
                                    Ok(_user) => _user,
                                    Err(err) => {
                                        tracing::error!("{:?}", err);
                                        return;
                                    },
                                };
                                
                                if let Err(err) = invite_user(&ctx.http, &pool, owner.get() as i64, &invited_user).await {
                                    tracing::error!("{:?}", err);
                                    return;    
                                };

                                SELECTED_USER_STORE.get().unwrap().remove(&owner);
                            } else {
                                if let Err(err) = mci.create_response(&ctx.http, CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content("⚠️ Choose a member!")
                                        .ephemeral(true)
                                )).await {
                                    tracing::error!("{:?}", err);
                                    return;    
                                };
                            }
                        }
                    },
                    _ => {
                        tracing::warn!("Unkown interaction id: {}", action_type);
                    }
                }
            }
        }
    }
}

fn configurate_logger() {
    let ingesting_host = std::env::var("BETTER_STACK_INGESTING_HOST")
        .expect("BETTER_STACK_INGESTING_HOST must be set");
    let source_token = std::env::var("BETTER_STACK_SOURCE_TOKEN")
        .expect("BETTER_STACK_SOURCE_TOKEN must be set");

    SELECTED_USER_STORE.set(DashMap::new()).unwrap();

    tracing_subscriber::registry()
        .with(BetterStackLayer::new(
            BetterStackConfig::builder(ingesting_host, source_token).build()
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .compact()
        )
        .with(EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .with_env_var("LOG_LEVEL")
            .from_env_lossy()
        )
        .init();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    configurate_logger();

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
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_INVITES;

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