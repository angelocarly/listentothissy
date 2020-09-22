# listentothissy
Discord bot that creates spotify playlists based on discord messages

## Configuration
This bot requires a few necessary environment variables in order to function. These are
- DISCORD_TOKEN - Discord bot token
- CLIENT_ID - spotify client id
- CLIENT_SECRET - spotify client secret
- REDIRECT_URI - spotify redirect uri

## Usage

### Link account
- Get an authorization url by sending `~link`
- Visit the url and authorize the bot
- Send the following url to the bot using `~link <url>`
