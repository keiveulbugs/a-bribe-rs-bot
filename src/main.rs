mod commands;
use poise::serenity_prelude::{self as serenity};
use std::sync::atomic::*;

// These Atomic bools are shared across the modules to interact between commands
pub static STOPBOOL: AtomicBool = AtomicBool::new(false);
pub static UPDATEBOOL: AtomicBool = AtomicBool::new(true);

type Error = Box<dyn std::error::Error + Send + Sync>;

#[macro_use]
//.env variables
extern crate dotenv_codegen;

//Constants
// Your Bot token
const DISCORD_TOKEN: &str = dotenv!("DISCORD_TOKEN");
// This should be your guild id.
const PRIVATEGUILDID: serenity::GuildId = serenity::GuildId(1052280052496216155);

async fn on_ready(
    ctx: &serenity::Context,
    ready: &serenity::Ready,
    framework: &poise::Framework<(), Error>,
) -> Result<(), Error> {
    // This registers commands for the bot, guild commands are instantly active on specified servers
    //
    // The commands you specify here only work in your own guild!
    // This is useful if you want to control your bot from within your personal server,
    // but dont want other servers to have access to it.
    // For example sending an announcement to all servers it is located in.
    let builder = poise::builtins::create_application_commands(&framework.options().commands);
    let _commands =
        serenity::GuildId::set_application_commands(&PRIVATEGUILDID, &ctx.http, |commands| {
            *commands = builder.clone();

            commands
        })
        .await;
    // To announce that the bot is online.
    println!("{} is connected!", ready.user.name);

    Ok(())
}

#[tokio::main]
async fn main() {
    // Build our client.
    let client = poise::Framework::builder()
        .token(DISCORD_TOKEN)
        .intents(serenity::GatewayIntents::GUILDS | serenity::GatewayIntents::GUILD_MEMBERS)
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::stop::stop(),
                commands::contract_update::contract_update(),
                commands::help::help(),
                commands::bribewatch::bribewatch(),
            ],
            ..Default::default()
        })
        .setup(|ctx, ready, framework| Box::pin(on_ready(ctx, ready, framework)))
        .build()
        .await
        .expect("Error creating client");

    // Start client, show error, and then ask user to provide bot secret as that is the most common cause for failure
    if let Err(why) = client.start().await {
        println!("Client error: {:?}\n\n**********************\nTry entering a working bot-secret in the .env file", why);
    }
}
