use crate::{Error, UPDATEBOOL};
use std::sync::atomic::Ordering;

// This command swaps an atomic bool to true.
// When the blockstream command fetches a new block and this bool is set to true, it updates the contracts.
#[poise::command(slash_command)]
pub async fn contract_update(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    UPDATEBOOL.swap(true, Ordering::Relaxed);
    ctx.say("Checking contract addresses, this can take a few seconds")
        .await?;
    Ok(())
}

// Sometimes I am amazed how a file can go from over a hundred lines to 1 meaningful line.
