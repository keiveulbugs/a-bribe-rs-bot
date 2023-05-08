use crate::Error;
use crate::commands::databasesetup::databasesetup;

use ethers::{
    contract::abigen,
    core::abi::AbiDecode,
    providers::{Http, Middleware, Provider},
    types::{Address, Chain, Filter, H160, H256, U256},
    utils::format_units,
};
use ethers_etherscan::account::InternalTxQueryOption;
use lazy_static::lazy_static;

use poise::serenity_prelude::{CacheHttp, UserId};
use serde::{Deserialize, Serialize};

use serenity::collector::component_interaction_collector::CollectComponentInteraction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::utils::Colour;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use surrealdb::engine::local::File;
use surrealdb::sql::Thing;
use surrealdb::Surreal;


// Struct for determining visibility of message later on.
#[derive(poise::ChoiceParameter, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Visibility {
    Private,
    Private_with_dm,
    Public,
    Public_with_dm,
}

//Option<Visibility>,

// Database setup command
#[poise::command(slash_command)]
pub async fn database(
    ctx: poise::Context<'_, (), Error>,
    #[description = "Channel to post updates"] startblock: Option<u64>,
    #[description = "Delete database"] delete: Option<bool>,
) -> Result<(), Error> {


    if startblock.is_some() {
        let delete = delete.unwrap_or(false);
        databasesetup(ctx, delete, startblock.unwrap()).await?;
    }

    Ok(())
}
