use crate::{Error, UPDATEBOOL};
use poise::serenity_prelude::UserId;
use std::sync::atomic::Ordering;

// This command swaps an atomic bool to true.
// When the blockstream command fetches a new block and this bool is set to true, it updates the contracts.
#[poise::command(slash_command, guild_only = true)]
pub async fn contract_update(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    let rolesofuser = ctx.author_member().await.unwrap().permissions;
    if !rolesofuser.unwrap().administrator()
        && ctx.author().id != UserId(397118394714816513)
        && ctx.author().id != UserId(320292370161598465)
    {
        ctx.say("You don't have enough rights").await?;
        return Ok(());
    }
    UPDATEBOOL.swap(true, Ordering::Relaxed);
    ctx.say("Checking contract addresses, this can take a few seconds")
        .await?;
    Ok(())
}

// Sometimes I am amazed how a file can go from over a hundred lines to 1 meaningful line.
