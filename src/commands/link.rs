
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::{generate_random_string, process_token};
use serenity::client::{Context};
use serenity::framework::standard::{Args, CommandResult, macros::{
    command,
}};
use serenity::model::channel::{Message};
use crate::util::update_cache;
use crate::ThissyContainer;

/*
 * Link spotify account to discord id
 */
#[command]
async fn link(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let thissy_data = data.get_mut::<ThissyContainer>().expect("Expected ThissyContainer in context");

    if args.message().is_empty() {

        // ~link command was empty, check if id is already linked
        match &thissy_data.spotify_map.get(&msg.author.id.0) {
            Some(_) => {
                msg.author.dm(&ctx.http, |m| m.content("Your account is already linked")).await?;
            }
            None => {
                // Craft an authorization url and dm it to the user
                let oauth = SpotifyOAuth::default()
                    .scope("playlist-modify-public")
                    .build();

                let state = generate_random_string(16);
                let auth_url = oauth.get_authorize_url(Some(&state), None);
                let message = format!("In order to link your account, go to the following url, allow access and send me the final url like this: ~link <url>\n{}", auth_url);
                msg.author.dm(&ctx.http, |m| m.content(message)).await?;
            }
        }
    } else {

        // A message was given with the ~link command, try to complete the authorization
        let mut oauth = SpotifyOAuth::default()
            .scope("playlist-modify-public")
            .build();

        let mut url: String = args.message().to_string();
        match process_token(&mut oauth, &mut url).await {
            Some(token_info) => {

                // Building the client credentials with the access token.
                let client_credential = SpotifyClientCredentials::default()
                    .token_info(token_info)
                    .build();

                // Initialize the Spotify client.
                let spotify = Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build();

                let user = spotify.current_user().await.unwrap();
                let welcome_msg = format!("Welcome {}, your account is linked!", user.display_name.unwrap_or(user.id));

                // Store spotify in context
                thissy_data.spotify_map.insert(msg.author.id.0, spotify);
                update_cache(thissy_data);

                // store_cache(thissy_data.spotify_map).await;
                msg.author.dm(&ctx.http, |m| m.content(welcome_msg)).await?;
                println!("Linked new account, id {}", msg.author.id.0);
            }
            None => {
                msg.author.dm(&ctx.http, |m| m.content("Failed to link account, did you enter the correct url? Try again by sending ~link")).await?;
            }
        }
    }

    Ok(())
}
