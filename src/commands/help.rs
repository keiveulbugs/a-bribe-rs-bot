use crate::Error;
use poise::serenity_prelude::UserId;
use serenity::{model::application::component::ButtonStyle, utils::Colour};
// Help command
#[poise::command(slash_command)]
pub async fn help(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    ctx.send(|b| {
        b.embed(|b| {
            b.description(
                "This bot watches the blockchain for Solid Lizard Finances to notify community members of bribes",
            )
            .title("About this bot")
            .colour(Colour::BLITZ_BLUE)
        })
        .ephemeral(true)
        .components(|b| {
            b.create_action_row(|b| {
                b.create_button(|b| {
                    b.label("Invite")
                        .url("https://discord.com/api/oauth2/authorize?client_id=1094733668431429662&permissions=2214610944&scope=bot")
                        .style(ButtonStyle::Link)
                })
            })
        })
    })
    .await?;
    // Change this id to the user that needs permissions to change the id.
    if ctx.author().id == UserId(397118394714816513) {
        poise::builtins::register_application_commands_buttons(ctx).await?;
    }

    Ok(())
}
