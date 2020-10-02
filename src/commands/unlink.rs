use serenity::client::{Context};
use serenity::framework::standard::{CommandResult, macros::{
    command,
}};
use serenity::model::channel::{Message};
use crate::util::update_cache;
use crate::ThissyContainer;

/*
 * Unlink spotify account from discord account
 * This will also purge all subscribed channels
 */
#[command]
async fn unlink(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let thissy_data = data.get_mut::<ThissyContainer>().expect("Expected ThissyContainer in context");

    // Purge all subscriptions
    let subs: Vec<u64> = thissy_data.get_subs_by_discord_user(&msg.author.id.0);
    for key in subs {
        thissy_data.subdata.remove(&key);
    }

    // Remove spotify token
    if thissy_data.spotify_map.contains_key(&msg.author.id.0) {
        thissy_data.spotify_map.remove(&msg.author.id.0);
        update_cache(thissy_data);
        msg.author.dm(&ctx.http, |m| m.content("Unlinked your spotify account and removed your subs.")).await?;
    } else {
        msg.author.dm(&ctx.http, |m| m.content("Your spotify account isn't linked yet, can't unlink.")).await?;
    }

    Ok(())
}
