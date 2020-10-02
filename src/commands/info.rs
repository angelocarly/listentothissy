use serenity::client::Context;
use serenity::framework::standard::{CommandResult, macros::{
    command,
}};
use serenity::model::channel::Message;

use crate::{ThissyContainer};
use crate::util::{create_spotify_playlist_link};

#[command]
async fn info(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let thissy_data = data.get_mut::<ThissyContainer>().expect("Expected SpotifyContainer in context");

    match thissy_data.subdata.get(&msg.channel_id.0) {
        Some(subdata) => {
            let spotify_link = create_spotify_playlist_link(&subdata.spotify_playlist.as_str());
            msg.reply(ctx, format!("This channel is connected to a playlist: {}", spotify_link)).await?;
        }
        None => {
            msg.reply(ctx, format!("There's no playlist connected to this channel")).await?;
        }
    }

    Ok(())
}
