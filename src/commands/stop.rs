use crate::{Error, STOPBOOL};
use std::sync::atomic::Ordering::Relaxed;
use poise::serenity_prelude::UserId;

// This command stops the bribe checking
#[poise::command(slash_command, guild_only = true)]
pub async fn stop(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    let rolesofuser = ctx.author_member().await.unwrap().permissions;
    if !rolesofuser.unwrap().administrator() && ctx.author().id != UserId(397118394714816513){
        return Ok(());
    }
    STOPBOOL.swap(true, Relaxed);
    ctx.send(|b| b.content("Stopping the bribebot. If everything goes well, you should soon see a message that the bribebot was stopped originating from the other command.").ephemeral(true)).await?;
    Ok(())
}
