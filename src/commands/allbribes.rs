use crate::{Error, DB};

use ethers::{
    contract::abigen,
    types::{Address, H160, H256, U256},
    utils::format_units,
};

use lazy_static::lazy_static;

use serde::{Deserialize, Serialize};

use serenity::collector::component_interaction_collector::CollectComponentInteraction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::utils::Colour;

use std::borrow::Cow;
use std::collections::HashMap;

use std::sync::Mutex;

use surrealdb::sql::Thing;

lazy_static! {
    static ref HASHMAPOFPOOLS: Mutex<HashMap<H160, String>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
}

//static mut HASHMAPOFPOOLS: HashMap<H160, String> = std::collections::HashMap::new();

#[derive(Debug, Serialize, Deserialize)]
struct Bribe {
    pooladdress: Address,
    tokenaddress: Address,
    poolname: Cow<'static, str>,
    tokenname: Cow<'static, str>,
    amount: U256,
    sender: Address,
    txhash: H256,
    block: u64,
    decimals: u64,
}
#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

abigen!(
    BribeContract,
    r#"[
        createGauge(address _pool)
    ]
    "#
);
abigen!(
    PoolContract,
    r#"[
        function name() external view returns (string)
    ]
    "#
);

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logos {
    pub tokens: Vec<Token>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub address: String,
    pub decimals: u64,
    #[serde(rename = "chainId")]
    pub chain_id: i64,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    #[serde(rename = "coingeckoId")]
    pub coingecko_id: Option<String>,
    #[serde(rename = "listedIn")]
    #[serde(default)]
    pub listed_in: Vec<String>,
}

pub async fn allbribes(ctx: poise::Context<'_, (), Error>, ephemeral: bool) -> Result<(), Error> {
    let bribevec: Vec<Bribe> = DB.select("bribe").await?;
    let pagelength = bribevec.len() / 20 + 1;
    if bribevec.is_empty() {
        ctx.send(|b| { b.content("There are no bribes in the database") }.ephemeral(true))
            .await?;
        return Ok(());
    }

    let mut pagevec: Vec<(String, String, bool)> = vec![];

    let mut pagecount = 0;

    for bribes in bribevec.iter().take(20) {
        let mut readableamount = match format_units(bribes.amount, bribes.decimals as u32) {
            Ok(val) => val,
            Err(_) => "Unknown".to_string(),
        };
        match readableamount.find('.') {
            Some(val) => {
                readableamount.truncate(val + 4);
            }
            None => readableamount = "Unknown".to_string(),
        };
        pagevec.push((bribes.tokenname.to_string(), readableamount, false));
    }

    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx.id());
    let next_button_id = format!("{}next", ctx.id());

    // Send the embed with the first page as content

    ctx.send(|b| {
        b.embed(|b| {
            b.description(format!(
                "**Bribes {}-{}/{}**",
                pagecount * 20,
                pagecount * 20 + pagevec.len(),
                bribevec.len()
            ))
            .fields(pagevec.clone())
            .footer(|f| f.text(format!("Page {}/{}", pagecount + 1, pagelength)))
            .colour(Colour::BLITZ_BLUE)
        })
        .ephemeral(ephemeral)
        .components(|b| {
            b.create_action_row(|b| {
                b.create_button(|b| b.custom_id(&prev_button_id).emoji('◀'))
                    .create_button(|b| b.custom_id(&next_button_id).emoji('▶'))
            })
        })
    })
    .await?;

    // Loop through incoming interactions with the navigation buttons

    while let Some(press) = CollectComponentInteraction::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(std::time::Duration::from_secs(3600))
        .await
    {

        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id && pagecount < pagelength -1 {
            pagecount += 1;
        } else if press.data.custom_id == next_button_id {
            pagecount = 0;
        } else if press.data.custom_id == prev_button_id && pagecount > 0 {
            pagecount -= 1;
        } else if press.data.custom_id == prev_button_id {
            pagecount = pagelength -1 ;
        } else {
            // This is an unrelated button interaction
            continue;
        }


        pagevec.clear();
        for bribes in bribevec.iter().skip(20 * pagecount).take(20) {
            let mut readableamount = match format_units(bribes.amount, bribes.decimals as u32) {
                Ok(val) => val,
                Err(_) => "Unknown".to_string(),
            };
            match readableamount.find('.') {
                Some(val) => {
                    readableamount.truncate(val + 4);
                }
                None => readableamount = "Unknown".to_string(),
            };
            pagevec.push((bribes.tokenname.to_string(), readableamount, false));
        }

        // Update the message with the new page contents
        press
            .create_interaction_response(ctx, |b| {
                b.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|b| {
                        b.embed(|b| {
                            b.description(format!(
                                "**Bribes {}-{}/{}**",
                                pagecount * 20,
                                pagecount * 20 + pagevec.len(),
                                bribevec.len()
                            ))
                            .fields(pagevec.clone())
                            .colour(Colour::BLITZ_BLUE)
                            .footer(|f| f.text(format!("Page {}/{}", pagecount + 1, pagelength)))
                        })
                    })
            })
            .await?;
    }

    Ok(())
}

pub async fn dmallbribes(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    ctx.send(|b| { b.content("Sending you a DM") }.ephemeral(true))
        .await?;

    let bribevec: Vec<Bribe> = DB.select("bribe").await?;
    let pagelength = bribevec.len() / 20 + 1;
    if bribevec.is_empty() {
        ctx.send(|b| { b.content("There are no bribes in the database") }.ephemeral(true))
            .await?;
        return Ok(());
    }

    let mut pagevec: Vec<(String, String, bool)> = vec![];

    let mut pagecount = 0;

    for bribes in bribevec.iter().take(20) {
        let mut readableamount = match format_units(bribes.amount, bribes.decimals as u32) {
            Ok(val) => val,
            Err(_) => "Unknown".to_string(),
        };
        match readableamount.find('.') {
            Some(val) => {
                readableamount.truncate(val + 4);
            }
            None => readableamount = "Unknown".to_string(),
        };
        pagevec.push((bribes.tokenname.to_string(), readableamount, false));
    }

    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx.id());
    let next_button_id = format!("{}next", ctx.id());

    // Send the embed with the first page as content

    ctx.author()
        .dm(ctx, |b| {
            b.embed(|b| {
                b.description(format!(
                    "**Bribes {}-{}/{}**",
                    pagecount * 20,
                    pagecount * 20 + pagevec.len(),
                    bribevec.len()
                ))
                .fields(pagevec.clone())
                .footer(|f| f.text(format!("Page {}/{}", pagecount + 1, pagelength)))
                .colour(Colour::BLITZ_BLUE)
            })
            .components(|b| {
                b.create_action_row(|b| {
                    b.create_button(|b| b.custom_id(&prev_button_id).emoji('◀'))
                        .create_button(|b| b.custom_id(&next_button_id).emoji('▶'))
                })
            })
        })
        .await?;

    // Loop through incoming interactions with the navigation buttons
        
    while let Some(press) = CollectComponentInteraction::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(std::time::Duration::from_secs(3600*24))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id && pagecount < pagelength -1 {
            pagecount += 1;
        } else if press.data.custom_id == next_button_id {
            pagecount = 0;
        } else if press.data.custom_id == prev_button_id && pagecount > 0 {
            pagecount -= 1;
        } else if press.data.custom_id == prev_button_id {
            pagecount = pagelength -1 ;
        } else {
            // This is an unrelated button interaction
            continue;
        }

        pagevec.clear();
        for bribes in bribevec.iter().skip(20 * pagecount).take(20) {
            let mut readableamount = match format_units(bribes.amount, bribes.decimals as u32) {
                Ok(val) => val,
                Err(_) => "Unknown".to_string(),
            };
            match readableamount.find('.') {
                Some(val) => {
                    readableamount.truncate(val + 4);
                }
                None => readableamount = "Unknown".to_string(),
            };
            pagevec.push((bribes.tokenname.to_string(), readableamount, false));
        }

        // Update the message with the new page contents
        press
            .create_interaction_response(ctx, |b| {
                b.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|b| {
                        b.embed(|b| {
                            b.description(format!(
                                "**Bribes {}-{}/{}**",
                                pagecount * 20,
                                pagecount * 20 + pagevec.len(),
                                bribevec.len()
                            ))
                            .fields(pagevec.clone())
                            .colour(Colour::BLITZ_BLUE)
                            .footer(|f| f.text(format!("Page {}/{}", pagecount + 1, pagelength)))
                        })
                    })
            })
            .await?;
    }

    Ok(())
}
