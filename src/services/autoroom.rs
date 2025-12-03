use futures::{StreamExt, stream::FuturesUnordered};
use serenity::all::{ ChannelId, Context, GuildChannel, Http, PermissionOverwrite, PermissionOverwriteType, Permissions, UserId};

use crate::sql::{pool::GLOBAL_SQL_POOL, prelude::MonitoredAutoRoom};


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
                "Cleanup db channels have been completed successfuly.
                Total cleaned {} channels, Not a guild {}, mismatch ids {}, discord channels removed {}",
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

// enum CleanUpCategoriesRecord {
//     Found(GuildChannel),
//     NotFound(i64)
// }

// pub async fn cleanup_categories_monitored_rooms(ctx: &Context) -> Result<(), String> {
//     tracing::info!("Starting categories cleanup monitored rooms");
//     let pool = GLOBAL_SQL_POOL.get().unwrap().get_pool();
//     let category_ids = AutoRoom::get_all_category_ids(&pool)
//         .await
//         .map_err(|err| err.to_string())?;

//     let mut tasks = FuturesUnordered::new();

//     let http = &ctx.http;
//     for category_id in category_ids {
//         let http = http.clone();
//         tasks.push(async move {
//             match ChannelId::new(category_id as u64).to_channel(http).await {
//                 Ok(c) => {
//                     if let Some(g) = c.guild() {
//                         return CleanUpCategoriesRecord::Found(g)
//                     };
//                     tracing::info!("Category({}) aren't a guild channel.", category_id);
//                     CleanUpCategoriesRecord::NotFound(category_id)
//                 },
//                 Err(err) => {
//                     tracing::info!("Category({}) not found.\nError: {}", category_id, err.to_string());
//                     CleanUpCategoriesRecord::NotFound(category_id)
//                 }
//             }
//         });
//     }

//     let mut categories_to_delete: Vec<i64> = Vec::new();
//     while let Some(result) = tasks.next().await {
//         match result {
//             CleanUpCategoriesRecord::NotFound(category_id) => {
//                 categories_to_delete.push(category_id);
//             },
//             CleanUpCategoriesRecord::Found(category) => {
//                 PIVO, here need to get all channels of each category
                    // let members = channel.members(cache).map_err(|err| err.to_string())?;
                    //     if members.len() > 0 {
                    //         continue;
                    //     };
//                     category.delete(&ctx).await.map_err(|err| err.to_string())?;
//                     categories_to_delete.push(category.id.get() as i64);
//                 }
//             },
//         };
//     }

//     if !categories_to_delete.is_empty() {
//         AutoRoom::delete(
//             &pool, 
//             AutoRoomDeleteStrategy::ManyByCategoryId(categories_to_delete)
//         )
//             .await
//             .map_err(|err| err.to_string())?;
//         tracing::info!("Removed {} categories", ids_to_delete.len());
//     }

//     Ok(())
// }