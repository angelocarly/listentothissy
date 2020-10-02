use serenity::client::Context;
use serenity::framework::standard::{CommandResult, macros::command};
use serenity::model::channel::Message;

use crate::ThissyContainer;
use crate::util::{update_cache};

/*
 * Remove all followed channels
 */
#[command]
async fn purge(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let thissy_data = data.get_mut::<ThissyContainer>().expect("Expected SpotifyContainer in context");

    let subs: Vec<u64> = thissy_data.get_subs_by_discord_user(&msg.author.id.0);

    if subs.is_empty() {
        msg.author.dm(&ctx.http, |m| m.content("You have no playlists connected.")).await?;
        return Ok(());
    }

    let sub_count = subs.len();
    for key in subs {
        thissy_data.subdata.remove(&key);
    }
    msg.author.dm(&ctx.http, |m| m.content(format!("Purged {} channel(s).", sub_count))).await?;
    update_cache(thissy_data);

    Ok(())
}
