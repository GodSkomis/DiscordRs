use serenity::all::{ChannelId, Context, UserId};

use crate::{commands::{autoroom::savedroom::SaveRoomCacheRecord, CommandContext}, voice::prelude::SavedRoomCacheType};


pub async fn add_guest_to_cache(ctx: &CommandContext<'_>, channel: &ChannelId, guest_id: &UserId) {
    let mut cache = ctx.data().savedroom_cache.lock().await;
    cache.entry(channel.get() as i64)
        .or_insert(
            SaveRoomCacheRecord::new(
                ctx.author().id.get() as i64,
                Some(guest_id.get() as i64)
            )
    );
}

pub async fn remove_guest_cache(ctx: &Context, channel_id: &i64) {
    let data = ctx.data.read().await;
    let mut cache = data.get::<SavedRoomCacheType>()
    .expect("Failed to get SavedRoomCache")
    .lock().await;
    cache.remove(channel_id);
}