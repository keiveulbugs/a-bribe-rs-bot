use crate::{Error, STOPBOOL};
use std::sync::atomic::Ordering::Relaxed;

// This command stops the bribe checking
#[poise::command(slash_command)]
pub async fn stop(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    STOPBOOL.swap(true, Relaxed);
    ctx.send(|b| b.content("Stopping the bribebot. If everything goes well, you should soon see a message that the bribebot was stopped originating from the other command.").ephemeral(true)).await?;
    Ok(())
}
