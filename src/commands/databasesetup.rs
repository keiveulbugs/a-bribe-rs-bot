use crate::Error;
use crate::DB;
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
const BRIBEFACTORY: &str = dotenv!("BRIBEFACTORY");
const ARBSCANKEY: &str = dotenv!("ARBSCAN");

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

pub async fn databasesetup(
    ctx: poise::Context<'_, (), Error>,
    delete: bool,
    startblock: u64,
) -> Result<(), Error> {
    let rolesofuser = ctx.author_member().await.unwrap().permissions;
    if !rolesofuser.unwrap().administrator()
        && ctx.author().id != UserId(397118394714816513)
        && ctx.author().id != UserId(320292370161598465)
    {
        ctx.say("You don't have enough rights to do this!").await?;
        return Ok(());
    }
    ctx.say("creating the database".to_string()).await?;

    // // starts database in a local file
    // let db = match Surreal::new::<File>("temp.db").await {
    //     Ok(val) => val,
    //     Err(_) => {
    //         panic!("Couldn't connect to the database")
    //     }
    // };
    // // connects to the database with the right namescheme and name
    // db.use_ns("bribebot").use_db("bribebotdb").await?;

    // Deletes the database when requested by the user
    if delete {
        let deletebribe: Vec<Record> = DB.delete("bribe").await?;
        ctx.send(|b| b.content(format!("Deleted {} data entries", deletebribe.len())))
            .await?;
    };
    // we do this because otherwise I had to update my copy pasted code from the other file:)
    let channel = ctx.channel_id();
    // This messagehandle is later used to update the message
    let mut messagehandle = channel
        .send_message(ctx.http(), |b| b.content("Starting setup!"))
        .await?;

    // We get all tokens in the git
    let response =
        reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
            .await?;
    let jsontoken: Logos = response.json().await?;
    let token = jsontoken.tokens;

    // Connect to the chain rpc provider, connect to the explorer, create a vec of all smart contract addresses.
    let provider = Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?;
    let client = Arc::new(&provider);
    let mut veccontracts = vec!["0x98A1De08715800801E9764349F5A71cBe63F99cc".parse::<H160>()?];
    let address: Address = BRIBEFACTORY.parse()?;
    let arbscanclient = ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;

    let internaltxvec = arbscanclient
        .get_internal_transactions(InternalTxQueryOption::ByAddress(address), None)
        .await?;

    // This goes over all transactions made by the bribe factory contract, and then checks for the create result type.
    // Doing this we can index all gauge contracts
    let mut count = 0;
    'tx: for tx in internaltxvec {
        if tx.result_type == "create" && tx.contract_address.value().is_some() {
            let ad = tx.contract_address.value().unwrap();
            veccontracts.push(*ad);
            let trans = provider.get_transaction(tx.hash).await?.unwrap();
            let input = trans.input;
            let call = match CreateGaugeCall::decode(&input) {
                Ok(val) => val,
                Err(_) => {
                    continue 'tx;
                }
            };

            let pool: Address = call.pool;

            let contract = PoolContract::new(pool, client.clone());
            let name = match contract.name().call().await {
                Ok(val) => val,
                Err(_) => "A new Bribe occurred!".to_string(),
            };
            HASHMAPOFPOOLS.lock().unwrap().insert(*ad, name);
            count += 1;
            if count % 10 == 0 {
                messagehandle
                    .edit(ctx.http(), |b| {
                        b.content(format!("Starting setup!\n{} Contracts indexed!", count))
                    })
                    .await?;
            }
        }
    }
    // Here we reuse before mentioned message handle to count how many contracts are found.
    messagehandle
        .edit(ctx.http(), |b| {
            b.content(format!(
                "*Found {} contracts to watch!*",
                veccontracts.len()
            ))
        })
        .await?;
    // We fetch blocks in batches of 100k. This is to avoid reaching the upper limit and getting a rpc error response.
    // Explanation of what each variable does:
    // Lastblock is only used to make sure the while loop does not loop further than the last block.
    // tempend is the last block of that iteration. It is by default 100k blocks away from the startblock.
    // The startblock is where that iteration starts, the first iteration it starts at the users input
    // stopbool stops the whileloop when we arrive at the current block. A last iteration is performed where tempend is equal to the current block.
    let lastblock = provider.get_block_number().await?;
    let mut tempend = startblock + 1000000;
    let mut startblock = startblock;
    let mut stopbool = true;

    while stopbool {
        // Check if we reached the last block.
        if tempend > lastblock.as_u64() {
            tempend = provider.get_block_number().await?.as_u64();
            stopbool = false;
        };

        // Fetches all blocks from startblock to tempend and sorts by the smart contracts and the claim rewards topic.
        let filter = Filter::new()
            .from_block(startblock)
            .to_block(tempend)
            .topic0(
                "0xf70d5c697de7ea828df48e5c4573cb2194c659f1901f70110c52b066dcf50826"
                    .parse::<H256>()?,
            )
            .address(veccontracts.clone());

        let logs = client.get_logs(&filter).await?;

        // Checks the logs of all transactions
        // Usable variables:
        // erctoken: Token address
        // fromaddress: Sender
        // Amount
        // txhash
        // poolname
        // tokenname
        'logs: for log in logs {
            let erctoken = Address::from(log.topics[2]);
            let fromaddress = Address::from(log.topics[1]);

            let amount = match U256::decode(log.data) {
                Ok(val) => val,
                Err(_) => {
                    continue 'logs;
                }
            };
            let txhash = match log.transaction_hash {
                Some(val) => val,
                None => {
                    continue 'logs;
                }
            };
            let blocknumber = match log.block_number {
                Some(number) => number.as_u64(),
                None => {
                    continue 'logs;
                }
            };

            // let blockresult = match provider.get_block(logblocknumber).await {
            //     Ok(val) => val,
            //     Err(_) => {
            //         continue 'logs;
            //     }
            // };
            // let block = match blockresult {
            //     Some(val) => val,
            //     None => {
            //         continue 'logs;
            //     }
            // };

            let poolname = match HASHMAPOFPOOLS.lock().unwrap().get(&log.address) {
                Some(val) => val.to_string(),
                _ => "Unknown".to_string(),
            };
            let mut decimals = 18;
            let mut tokenname = "Unknown".to_string();
            if let Some(tokenstruct) = token
                .iter()
                .find(|p| p.address.to_lowercase() == format!("0x{:x}", erctoken))
            {
                tokenname = tokenstruct.name.clone();
                decimals = tokenstruct.decimals;
            };

            // database entry
            let _querycreation: Bribe = DB
                .create("bribe")
                .content(Bribe {
                    pooladdress: log.address,
                    tokenaddress: erctoken,
                    poolname: poolname.into(),
                    tokenname: tokenname.into(),
                    amount,
                    sender: fromaddress,
                    txhash,
                    block: blocknumber,
                    decimals,
                })
                .await?;
            //dbg!(_querycreation);
        }
        startblock = tempend + 1;
        tempend += 1000000;
    }

    let bribevec: Vec<Bribe> = DB.select("bribe").await?;
    let pagelength = bribevec.len() / 20 + 1;
    if bribevec.is_empty() {
        ctx.send(|b| {b.content("There are no bribes in the database")}.ephemeral(true)).await?;
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
        if press.data.custom_id == next_button_id && pagecount <= pagelength {
            pagecount += 1;
        } else if press.data.custom_id == next_button_id {
            ctx.send(|b| { b.content("You are on the last page") }.ephemeral(true))
                .await?;
            continue;
        } else if press.data.custom_id == prev_button_id && pagecount > 0 {
            pagecount -= 1;
        } else if press.data.custom_id == prev_button_id {
            ctx.send(|b| { b.content("You are on the first page") }.ephemeral(true))
                .await?;
            continue;
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
