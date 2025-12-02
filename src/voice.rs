use serenity::model::voice::VoiceState;
use serenity::model::id::ChannelId;
use serenity::builder::CreateChannel;
use serenity::client::Context;
use serenity::model::channel::{ Message, Channel };

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

pub async fn remove_proccessing(ctx: &Context, new: &VoiceState) {
    if let Some(channel_id) = &new.channel_id {
        let data = ctx.data.read().await;
        let pool = data.get::<SerenityPool>().expect("Failed to get DB pool");
         tracing::info!("Remove Room: {}", channel_id.get() as i64);
        if !MonitoredAutoRoom::exists(pool, channel_id.get() as i64).await {
            return;
        };

        match channel_id.to_channel(&ctx.http).await {
            Ok(channel) => {
                // if let Some(members) = &channel.members(&ctx.http).await.unwrap() {
                match &channel.clone().guild().unwrap().members(&ctx.cache) {
                    Ok(members) => {
                        if members.len() == 0 {
                            let _ = match channel.delete(&ctx.http).await {
                                Ok(_) => {let _ = MonitoredAutoRoom::remove(pool, channel_id.get() as i64).await;},
                                Err(err) => tracing::error!("Remove Room `MonitoredAutoRoom` Error: {}", err),
                            };
                        };
                    },
                    Err(err) => tracing::error!("Remove Room `members` Error: {}", err)
        }},
            Err(err) => tracing::error!("Remove Room `ctx.http` Error: {}", err)
        }
    }
}

pub struct VoiceProccessing;

impl VoiceProccessing {
    pub async fn proccess(&self, ctx: &Context, msg: &Message) -> Option<String>{
        if !msg.content.starts_with("!autoroom") {
            return None;
        };
        let commands: Vec<&str> = msg.content.split(' ').collect();
        if commands.len() < 4 {
            return Some(format!("Wrong number of arguments: {}", commands.len()));
        };
        match commands[1] {
            "add" => return Some(self.proccess_add(ctx, msg, commands).await),
            "list" => return Some(String::from("NotImplemented")),
            _ => return Some(String::from("Wrong subcommand"))
        };
    }

    async fn proccess_add(&self, ctx: &Context, msg: &Message, commands: Vec<&str>) -> String {
        let data = ctx.data.read().await;
        let pool = data.get::<SerenityPool>().expect("Failed to get DB pool");
        let channel_id = ChannelId::new(commands[2].parse::<u64>().unwrap());
        let category_id = ChannelId::new(commands[3].parse::<u64>().unwrap());
        let suffix = match commands.get(4) {
            Some(suffix) => suffix,
            None => "room",
        };
        if !self.check_channel(ctx, msg, &channel_id).await {
            return format!("Wrong channel id: {}", channel_id);
        }
        if !self.check_channel(ctx, msg, &category_id).await {
            return format!("Wrong category id: {}", category_id);
        }
        let autoroom = AutoRoom { channel_id: channel_id.get() as i64, category_id: category_id.get() as i64, suffix: suffix.to_string() };
        autoroom.create(pool).await;
        return format!("Record was created! channel id: {}, category id: {}", channel_id, category_id);
    }

    async fn check_channel(&self, ctx: &Context, msg: &Message, channel_id: &ChannelId) -> bool {
        if let Some(channel) = self.get_channel(ctx, channel_id).await {
            if let Some(guild_channel) = channel.guild() {
                if let Some(guild_id) = msg.guild_id {
                    return guild_channel.guild_id == guild_id;
                }
            }
        }
        return false;
    }

    async fn get_channel(&self, ctx: &Context, channel_id: &ChannelId) -> Option<Channel> {
    if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
        return Some(channel);
    };
    None
}
}