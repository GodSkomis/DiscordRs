use serenity::all::{Cache, Http};
use serenity::model::voice::VoiceState;
use serenity::model::id::ChannelId;
use serenity::builder::CreateChannel;
use serenity::client::Context;

use crate::services::autoroom::grant_owner_privileges;

use super::sql::SerenityPool;
use super::sql::autoroom::{AutoRoom, MonitoredAutoRoom};

use super::bitrate::get_bitrate;


pub async fn create_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = new.channel_id {

        let data = ctx.data.read().await;
        let pool = data.get::<SerenityPool>().expect("Failed to get DB pool");

        let autoroom_result = AutoRoom::get_by_channel_id(pool, channel_id.get() as i64).await;
        let autoroom = match autoroom_result{
            Ok(Some(autoroom)) => {
                autoroom
            }
            Ok(None) => {
                return;
            }
            Err(e) => {
                tracing::error!("Error fetching autoroom: {:?}", e);
                return;
            }
        };
            
        if let Some(guild_id) = new.guild_id {
            // Get max available server bitrate
            let max_bitrate = get_bitrate(&guild_id.to_guild_cached(&ctx.cache).unwrap().premium_tier);

            if let Some(member) = &new.member {
                let user_id = member.user.id;
                let user_name = &member.user.name;

                // Создаем новый голосовой канал с именем пользователя
                let builder = CreateChannel::new(format!("{}`s {}", user_name, autoroom.suffix))
                    .category(ChannelId::new(autoroom.category_id as u64))
                        .kind(serenity::model::channel::ChannelType::Voice)
                            .bitrate(max_bitrate);
                let channel_result = guild_id.create_channel(&ctx.http, builder).await;

                if let Ok(channel) = channel_result {
                    // Устанавливаем разрешения для пользователя
                    if let Err(_) = grant_owner_privileges(&ctx.http, &channel.id, &user_id).await {
                        let _ = channel.delete(&ctx.http).await;
                    }

                    // Переносим пользователя в созданный канал
                    if let Err(why) = guild_id.move_member(&ctx.http, user_id, channel.id).await {
                        tracing::error!(
                            "Failed to move the user({:?}) to the new voice channel({:?}). Error: \"{:?}\"",
                            &user_id.get(),
                            &channel.id.get(),
                            &why
                        );
                    }

                    MonitoredAutoRoom::new(
                        pool, channel.id.get() as i64,
                        user_id.get() as i64
                    ).await;
                    
                }
            }
        }
    }
}

pub async fn remove_channel_by_voicestate(ctx: &Context, new: &VoiceState) -> Result<(), String> {
    if let Some(channel_id) = new.channel_id {
        let data = ctx.data.read().await;
        let pool = data.get::<SerenityPool>().expect("Failed to get DB pool");
        if !MonitoredAutoRoom::exists(pool, channel_id.get() as i64).await {
            return Ok(());
        };
        
        tracing::info!("Remove Room: {}", channel_id.get() as i64);

        match channel_id.to_channel(&ctx.http).await {
            Ok(channel) => {
                match &channel.clone().guild().unwrap().members(&ctx.cache) {
                    Ok(members) => {
                            if members.len() == 0 {
                                let _ = match channel.delete(&ctx.http).await {
                                    Ok(_) => {
                                        MonitoredAutoRoom::remove(pool, channel_id.get() as i64)
                                            .await
                                            .map_err(|err| err.to_string())?;
                                    },
                                    Err(err) => {
                                        tracing::error!("Remove Room `MonitoredAutoRoom` Error: {}", err);
                                        return Err(err.to_string())
                                    }   
                                };
                            };
                            return Ok(())
                    },
                    Err(err) => {
                            tracing::error!("Remove Room `members` Error: {}", err);
                            return Err(err.to_string())
                    }
                }
            },
            Err(err) => {
                        tracing::error!("Remove Room `ctx.http` Error: {}", err);
                        return Err(err.to_string())
            }  
        };
    };

    Ok(())
}

#[allow(dead_code)]
pub async fn remove_channel_by_id_proccessing(
    http: &Http, cache: &Cache, channel_id: &ChannelId, pool: &sqlx::Pool<sqlx::Postgres>
) -> Result<(), String> {
    match channel_id.to_channel(http).await {
        Ok(channel) => {
            // if let Some(members) = &channel.members(&ctx.http).await.unwrap() {
            match &channel.clone().guild().unwrap().members(cache) {
                Ok(members) => {
                    if members.len() == 0 {
                        let _ = match channel.delete(http).await {
                            Ok(_) => {
                                MonitoredAutoRoom::remove(pool, channel_id.get() as i64)
                                    .await
                                    .map_err(|err| err.to_string())?;
                            },
                            Err(err) => {
                                tracing::error!("Remove Room `MonitoredAutoRoom` Error: {}", err);
                                return Err(err.to_string())
                            }
                        };
                    };
                },
                Err(err) => {
                    tracing::error!("Remove Room `members` Error: {}", err);
                    return Err(err.to_string())
                }
        }},
            Err(err) => {
                tracing::error!("Remove Room `ctx.http` Error: {}", err);
                return Err(err.to_string())
            }
    }

    Ok(())
}