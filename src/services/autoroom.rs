use std::collections::{HashMap, HashSet};

use futures::{StreamExt, stream::FuturesUnordered};
use serenity::all::{ ChannelId, Context, Guild, GuildChannel, Http, PermissionOverwrite, PermissionOverwriteType, Permissions, UserId};

use crate::sql::{autoroom::AutoRoomDeleteStrategy, pool::GLOBAL_SQL_POOL, prelude::{AutoRoom, MonitoredAutoRoom}};


pub async fn  grant_owner_privileges(http: &Http, channel: &ChannelId, user_id: &UserId) -> Result<(), serenity::Error> {
    let permissions = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::MANAGE_CHANNELS
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(*user_id),
    };
    if let Err(err) = channel.create_permission(http, permissions).await {
        tracing::error!(
            "Failed to grant channel({:?}) permissions to the user({:?}). Error: \"{:?}\"",
            &channel.get(),
            &user_id.get(),
            &err
        );
        return Err(err);
    }
    Ok(())
}

pub async fn  grant_guest_privileges(http: &Http, channel: &ChannelId, user_id: &UserId) -> Result<(), serenity::Error> {
    let permissions = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(*user_id),
    };
    if let Err(err) = channel.create_permission(http, permissions).await {
        tracing::error!(
            "Failed to grant channel({:?}) permissions to the user({:?}). Error: \"{:?}\"",
            &channel.get(),
            &user_id.get(),
            &err
        );
        return Err(err);
    }
    Ok(())
}


pub mod voice_channel {
    use serenity::all::{ChannelId, Http, User};

    use crate::sql::{pool::PoolType, prelude::MonitoredAutoRoom};
    use super::grant_guest_privileges;

    #[derive(thiserror::Error, Debug)]
    pub enum InviteError {
        #[error("The connected voice channel was not found")]
        MonitoredAutoRoomNotFound,

        #[error("Internal server error. Please try again later")]
        DatabaseError,

        #[error("Could not reach Discord. Please try again later")]
        SerenityError,
    }

    pub async fn invite_user(http: &Http, pool: &PoolType, author_id: i64, invited_user: &User) -> Result<(), InviteError> {
        let monitored_autoroom = match MonitoredAutoRoom::get_by_owner_id(pool, author_id).await {
            Ok(option) => match option {
                Some(monitored_autoroom_result) => monitored_autoroom_result,
                None => return Err(InviteError::MonitoredAutoRoomNotFound)
            },
            Err(err) => {
                tracing::error!("invite_user database error AUTHOR({}) INVITED({}).\n{}", author_id, invited_user, err);
                return Err(InviteError::DatabaseError)
            },
        };

        let channel_id = ChannelId::new(monitored_autoroom.channel_id as u64);
        grant_guest_privileges(http, &channel_id, &invited_user.id)
            .await
            .map_err(|err| {
                tracing::error!("invite_user serenity error AUTHOR({}) INVITED({}).\n{}", author_id, invited_user, err);
                InviteError::SerenityError
            })?;

        Ok(())
    }
}

struct CleanUpDbRecord {
    channel: Option<GuildChannel>,
    autoroom: MonitoredAutoRoom
}

#[derive(Default, Debug)]
struct CleanUpDbResult {
    pub not_a_guild_channel: Vec<i64>,
    pub not_match_ids: Vec<i64>,
    pub are_empty: Vec<GuildChannel>
}

impl CleanUpDbResult {
    pub fn outdated(&self) -> Vec<i64> {
        self.not_a_guild_channel
            .iter()
            .copied()
            .chain(self.not_match_ids.iter().copied())
            .collect()
    }
}


pub async fn cleanup_db_monitored_rooms(ctx: &Context) -> Result<(), String> {
    tracing::info!("Starting cleanup monitored rooms");
    let pool = GLOBAL_SQL_POOL.get().unwrap().get_pool();
    let autorooms = MonitoredAutoRoom::get_all(&pool)
        .await
        .map_err(|err| err.to_string())?;

    tracing::info!("Total monitored rooms {}", autorooms.len());

    let mut cleanup_result = CleanUpDbResult::default();
    let mut tasks = FuturesUnordered::new();
    let http = &ctx.http;
    let cache = &ctx.cache;

    for room in autorooms {

        tasks.push(async move {
            let channel = match ChannelId::new(room.channel_id as u64).to_channel(http).await {
                Ok(c) => c,
                Err(_) => return None,
            };
            
            Some(CleanUpDbRecord {
                channel: channel.guild(),
                autoroom: room
            })
        });
    };

    while let Some(result) = tasks.next().await {
        if let Some(record) = result {
            let autoroom = record.autoroom;
            let channel = match record.channel {
                Some(_c) => _c,
                None => {
                    cleanup_result.not_a_guild_channel.push(autoroom.channel_id);
                    continue;
                },
            };
            if channel.id.get() != autoroom.channel_id as u64 {
                cleanup_result.not_match_ids.push(autoroom.channel_id);
                continue;
            };

            let members = channel.members(cache).map_err(|err| err.to_string())?;
            if members.len() > 0 {
                continue;
            };
            cleanup_result.are_empty.push(channel);
        };
    };

    for channel in &cleanup_result.are_empty {
        match channel.delete(http).await {
            Ok(_) => (),
            Err(_err) => tracing::error!("Error to delete channel ({}).\nError: {}", channel.id.get(), _err)
        }
    }

    let are_empty_ids = cleanup_result.are_empty
            .iter()
            .map(|c| c.id.get() as i64)
            .collect();
    let ids_to_delete = [cleanup_result.outdated(), are_empty_ids].concat();

    match MonitoredAutoRoom::remove_many(&pool, &ids_to_delete).await {
        Ok(_) => {
            tracing::info!(
                "[Cleanup DB] Completed | Cleaned: {} | Not a guild: {} | Mismatch IDs: {} | Discord removed: {}",
                ids_to_delete.len(),
                cleanup_result.not_a_guild_channel.len(),
                cleanup_result.not_match_ids.len(),
                cleanup_result.are_empty.len()
            );
            Ok(())
        },
        Err(err) => Err(err.to_string()),
    }

}


enum CleanUpCategoriesRecord {
    Found(GuildChannel),
    NotFound(i64)
}

struct CleanUpCategoriesGuild {
    pub guild: Guild,
    pub category_ids: HashSet<u64>
}

impl CleanUpCategoriesGuild {
    pub fn new(guild:Guild, category_id: u64) -> Self {
        let mut set = HashSet::new();
        set.insert(category_id);
        Self {
            guild: guild,
            category_ids: set,
        }
    }
}

pub async fn cleanup_categories_monitored_rooms(ctx: &Context) -> Result<(), String> {
    tracing::info!("Starting categories cleanup monitored rooms");
    let pool = GLOBAL_SQL_POOL.get().unwrap().get_pool();
    let category_ids = AutoRoom::get_all_category_ids(&pool)
        .await
        .map_err(|_err| _err.to_string())?;
    tracing::info!("Total category count in autoroom {}", category_ids.len());
    let mut tasks = FuturesUnordered::new();

    let http = &ctx.http;
    for category_id in category_ids {
        let http = http.clone();
        tasks.push(async move {
            match ChannelId::new(category_id as u64).to_channel(http).await {
                Ok(c) => {
                    if let Some(g) = c.guild() {
                        if g.id.get() as i64 != category_id {
                            tracing::error!(
                                "[Category id missmatch]: [Discord({})] |[Db({})]",
                                g.id.get(),
                                category_id,
                            );
                            return CleanUpCategoriesRecord::NotFound(category_id)
                        };
                        return CleanUpCategoriesRecord::Found(g)
                    };
                    CleanUpCategoriesRecord::NotFound(category_id)
                },
                Err(err) => {
                    tracing::error!("Category({}) not found.\nError: {}", category_id, err.to_string());
                    CleanUpCategoriesRecord::NotFound(category_id)
                }
            }
        });
    }

    let mut outdated_categories: Vec<i64> = Vec::new();
    let mut guilds: HashMap<u64, CleanUpCategoriesGuild> = HashMap::default();
    while let Some(result) = tasks.next().await {
        match result {
            CleanUpCategoriesRecord::NotFound(category_id) => {
                outdated_categories.push(category_id);
            },
            CleanUpCategoriesRecord::Found(category) => {
                let guild: Guild = (*category
                // let guild: CacheRef<'_, GuildId, Guild, Infallible> = category
                    .guild(ctx)
                    .expect("GuildChannel without guild")).clone();
                let guild_id = guild.id.get().clone();
                guilds.entry(guild_id)
                    .or_insert_with(
                        ||
                        CleanUpCategoriesGuild::new(guild, category.id.get())
                    )
                    .category_ids
                    .insert(category.id.get());
            }
        }
    }

    if !outdated_categories.is_empty() {
        AutoRoom::delete(
            &pool, 
            AutoRoomDeleteStrategy::ManyByCategoryId(&outdated_categories)
        )
            .await
            .map_err(|err| err.to_string())?;
        tracing::info!("Removed ({}) outdated categories", outdated_categories.len());
    }

    if !guilds.is_empty() {
        let channels: Vec<&GuildChannel> = guilds
            .values()
            .flat_map(
                |g| g.guild.channels
                    .iter()
                    .map(|c| c.1)
                    .filter(|c| c.parent_id.is_some())
                    .filter(|c| g.category_ids.contains(&c.parent_id.unwrap().get()))
            )
            .collect();

        let mut autorooms_to_insert: Vec<MonitoredAutoRoom> = Vec::new();
        let mut channels_to_delete: Vec<&GuildChannel> = Vec::new();
        let bot_id = ctx.cache.current_user().id;
        for channel in channels {
            let members = channel.members(ctx).map_err(|err| err.to_string())?;
            if members.len() > 0 {
                autorooms_to_insert.push(MonitoredAutoRoom {
                    channel_id: channel.id.get() as i64,
                    owner_id: (channel.owner_id.unwrap_or(bot_id)).get() as i64
                });
                continue;
            }
            channels_to_delete.push(&channel);

        }

        tracing::info!("[Cleanup categories] {} rooms to delete", channels_to_delete.len());
        for channel in &channels_to_delete {
            match channel.delete(ctx).await {
                Ok(_) => (),
                Err(err) => tracing::error!(
                    "[Cleanup categories] channel({}) delete error:\n`{}`",
                    channel.id.get(),
                    err
                )
            };
        }
        tracing::info!("[Cleanup categories] {} rooms removed", channels_to_delete.len());

        tracing::info!("[Cleanup categories] {} rooms to create", autorooms_to_insert.len());
        MonitoredAutoRoom::insert_many(&pool, &autorooms_to_insert).await.map_err(|err| err.to_string())?;
        tracing::info!("[Cleanup categories] {} rooms created", autorooms_to_insert.len());
    }
    
    tracing::info!("[Cleanup categories] monitored rooms have been completed");
    Ok(())
}