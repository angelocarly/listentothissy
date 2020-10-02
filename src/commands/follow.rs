use serenity::client::{Context};
use serenity::framework::standard::{Args, CommandResult, macros::{
    command,
    hook,
}, StandardFramework};
use serenity::model::channel::{Message};
use crate::{SubscribeData, ThissyContainer};
use crate::util::{update_cache, is_valid_token, get_refresh_credentials};
use rspotify::client::Spotify;

#[command]
async fn follow(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let thissy_data = data.get_mut::<ThissyContainer>().expect("Expected SpotifyContainer in context");

    match thissy_data.spotify_map.get_mut(msg.author.id.as_u64()) {
        Some(mut spotify) => {

            // Verify token validity
            let mut val: Spotify;
            if !is_valid_token(&spotify) {
                val = get_refresh_credentials(&spotify).await;
                spotify = &mut val;
            }

            // Verify if playlist is the correct format
            let mut playlist = args.message().to_string();
            if !playlist.starts_with("spotify:playlist:") {
                msg.author.dm(&ctx.http, |m| m.content("Your playlist URI is required, this is in the format spotify:playlist:xxxxxxxx")).await?;
                return Ok(());
            }

            // Verify token validity and if user owns the playlist
            match spotify.me().await {
                Ok(user) => {

                    let spotify_id = user.id;
                    let playlist_id = spotify.user_playlist(&spotify_id, Some(&mut playlist), None, None).await.unwrap().id;
                    if !playlist.ends_with(playlist_id.as_str()) {
                        msg.author.dm(&ctx.http, |m| m.content("You do not own this playlist")).await?;
                        return Ok(());
                    }

                    // Insert subscribe data
                    let subscribe = SubscribeData {
                        discord_user: msg.author.id.0,
                        spotify_user: spotify_id,
                        spotify_playlist: playlist_id,
                    };

                    // Update context
                    thissy_data.subdata.insert(msg.channel_id.0, subscribe);
                    update_cache(thissy_data);

                    msg.author.dm(&ctx.http, |m| m.content("Successfully followed text channel!")).await?;

                },
                // User data could not be retrieved
                Err(e) => {
                    msg.author.dm(&ctx.http, |m| m.content(format!("An error occurred:{}", e.to_string()))).await?;
                }
            }
        }
        None => {
            msg.author.dm(&ctx.http, |m| m.content("Your account is not linked yet. Message ~link to link it.")).await?;
        }
    }


    Ok(())
}
