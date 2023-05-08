use crate::{Error, STOPBOOL, UPDATEBOOL};
use chrono::{prelude::Utc, DateTime};
use ethers::{
    contract::abigen,
    core::abi::AbiDecode,
    providers::{Http, Middleware, Provider},
    types::{Address, Chain, Filter, H160, H256, U256},
    utils::format_units,
};
use ethers_etherscan::account::InternalTxQueryOption;
use poise::serenity_prelude::{Activity, CacheHttp, ChannelId, UserId};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::sync::Arc;
use std::{collections::HashMap, sync::atomic::Ordering::Relaxed};

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
    pub decimals: i64,
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

// If you want to use alchemy instead of the public rpc, enable this and line 87.
// const ALCHEMYKEY: &str = dotenv!("ALCHEMY");

// Command that starts watching all blocks for contract interaction
#[poise::command(slash_command, guild_only = true)]
pub async fn bribewatch(
    ctx: poise::Context<'_, (), Error>,
    #[description = "Channel to post updates"] channel: ChannelId,
    // #[description = "Fetch the total amount of bribes"]    total: bool,
) -> Result<(), Error> {
    let rolesofuser = ctx.author_member().await.unwrap().permissions;
    if !rolesofuser.unwrap().administrator()
        && ctx.author().id != UserId(397118394714816513)
        && ctx.author().id != UserId(320292370161598465)
    {
        ctx.say("You don't have enough rights to do this!").await?;
        return Ok(());
    }
    ctx.say(format!(
        "Starting the bot, check <#{}> for more info!",
        channel
    ))
    .await?;

    let mut messagehandle = channel
        .send_message(ctx.http(), |b| b.content("Starting setup!"))
        .await?;
    let response =
        reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
            .await?;
    let jsontoken: Logos = response.json().await?;

    let mut token = jsontoken.tokens;

    // Uncomment this if you want to use alchemy instead of the default rpc.
    // let provider = Provider::<Ws>::connect(format!("wss://arb-mainnet.g.alchemy.com/v2/{}", ALCHEMYKEY))
    // .await
    // .map_err(|wserr| format!("Couldn't connect to the Alchemy websocket! {}", wserr))?;
    let provider = Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?;
    let client = Arc::new(&provider);
    let mut veccontracts = vec!["0x98A1De08715800801E9764349F5A71cBe63F99cc".parse::<H160>()?];
    let address: Address = BRIBEFACTORY.parse()?;
    let arbscanclient = ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;

    let mut hashmapofpools: HashMap<H160, String> = std::collections::HashMap::new();

    UPDATEBOOL.swap(false, Relaxed);
    let internaltxvec = arbscanclient
        .get_internal_transactions(InternalTxQueryOption::ByAddress(address), None)
        .await?;
    let mut count = 0;
    'tx: for tx in internaltxvec {
        if tx.result_type == "create" && tx.contract_address.value().is_some() {
            let ad = tx.contract_address.value().unwrap();
            veccontracts.push(*ad);
            //let input = tx.input;
            //println!("{:#?}", &tx);
            let trans = provider.get_transaction(tx.hash).await?.unwrap();
            let input = trans.input;
            let call = match CreateGaugeCall::decode(&input) {
                Ok(val) => val,
                Err(_) => {
                    continue 'tx;
                }
            };

            let pool: Address = call.pool;

            //let contract = IERC20::new(address, client);
            let contract = PoolContract::new(pool, client.clone());
            let name = match contract.name().call().await {
                Ok(val) => val,
                Err(_) => "A new Bribe occurred!".to_string(),
            };

            hashmapofpools.insert(*ad, name);
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
    messagehandle
        .edit(ctx.http(), |b| {
            b.content(format!("Starting setup!\n{} Contracts indexed!", count))
        })
        .await?;

    let response =
        reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
            .await?;
    let jsontoken: Logos = response.json().await?;
    token = jsontoken.tokens;

    ctx.channel_id()
        .say(
            ctx,
            format!("*Found {} contracts to watch!*", veccontracts.len()),
        )
        .await?;

    let _hashmapofamount: HashMap<H160, U256> = std::collections::HashMap::new();

    // if total {
    //     ctx.say("Getting the total amount of bribes. Note this may take a long time.").await?;

    // };

    // Change the 1000 to go back further in time on use of the slash command.
    // Now it fetches about 5 minutes of previous blocks to see if there are bribes.
    let mut lastblock = provider.get_block_number().await? - 1000;

    'mainloop: loop {
        let currenttime = tokio::time::Instant::now();
        let currentblock = match provider.get_block_number().await {
            Ok(val) => val,
            Err(_) => match provider.get_block_number().await {
                Ok(val) => val,
                Err(_) => {
                    continue 'mainloop;
                }
            },
        };
        let status = format!("block {}", currentblock);
        poise::serenity_prelude::Context::set_activity(
            ctx.serenity_context(),
            Activity::watching(status),
        )
        .await;

        let filter = Filter::new()
            .to_block(currentblock)
            .from_block(lastblock)
            .topic0(
                "0xf70d5c697de7ea828df48e5c4573cb2194c659f1901f70110c52b066dcf50826"
                    .parse::<H256>()?,
            )
            .address(veccontracts.clone());

        let logs = client.get_logs(&filter).await?;
        // println!("{} transactions found!", logs.iter().len());
        'logs: for log in logs {
            let erctoken = Address::from(log.topics[2]);
            let fromaddresstest = Address::from(log.topics[1]);
            let fromadress = if fromaddresstest
                == "0x98A1De08715800801E9764349F5A71cBe63F99cc".parse::<H160>()?
            {
                "Solid Lizard team!".to_string()
            } else {
                format!("0x{:X}", fromaddresstest)
            };

            let amount = match U256::decode(log.data) {
                Ok(val) => val,
                Err(_) => {
                    continue 'logs;
                }
            };
            let tx = match log.transaction_hash {
                Some(val) => val,
                None => {
                    continue 'logs;
                }
            };
            let logblocknumber = match log.block_number {
                Some(val) => val,
                None => {
                    continue 'logs;
                }
            };

            let blockresult = match provider.get_block(logblocknumber).await {
                Ok(val) => val,
                Err(_) => {
                    continue 'logs;
                }
            };
            let block = match blockresult {
                Some(val) => val,
                None => {
                    continue 'logs;
                }
            };

            let time = block.timestamp;

            // The old way of getting the utc from the time is a lot cleaner, however, a new way is needed as seen below to avoid it crashing when we go over 262 000 years.
            //let utc = chrono::Utc.timestamp(time.low_u64() as i64, 0);
            let utc = DateTime::<Utc>::from_utc(
                match chrono::NaiveDateTime::from_timestamp_opt(time.low_u64() as i64, 0) {
                    Some(val) => val,
                    None => {
                        continue 'logs;
                    }
                },
                Utc,
            );

            let poolname = match hashmapofpools.get(&log.address) {
                Some(val) => val,
                _ => "A new Bribe occurred",
            };

            if let Some(tokenname) = token
                .iter()
                .find(|p| p.address.to_lowercase() == format!("0x{:x}", erctoken))
            {
                let imageurl = tokenname
                    .logo_uri
                    .clone()
                    .ok_or("https://solidlizard.finance/images/ui/lz-logo.png".to_string())?;

                let decimals = tokenname.decimals;

                let mut readableamount = match format_units(amount, decimals as u32) {
                    Ok(val) => val,
                    Err(_) => "Unknown".to_string(),
                };
                match readableamount.find('.') {
                    Some(val) => {
                        readableamount.truncate(val + 4);
                    }
                    None => readableamount = "Unknown".to_string(),
                };

                channel
                    .send_message(ctx.http(), |a| {
                        a.embed(|b| {
                            b.title(poolname)
                                .url(format!("https://arbiscan.io/tx/0x{:x}", tx))
                                .field("Bribe creator", fromadress, false)
                                .field("Token", tokenname.name.clone(), false)
                                .field("Amount", readableamount, false)
                                .thumbnail(imageurl)
                                .footer(|f| {
                                    f.text("Sliz productions".to_string()).icon_url(
                                        "https://solidlizard.finance/images/ui/lz-logo.png",
                                    )
                                })
                                .timestamp(utc)
                        })
                    })
                    .await?;
            } else {
                let mut readableamount = match format_units(amount, "ether") {
                    Ok(val) => val,
                    Err(_) => "Unknown".to_string(),
                };
                match readableamount.find('.') {
                    Some(val) => {
                        readableamount.truncate(val + 4);
                    }
                    None => readableamount = "Unknown".to_string(),
                };

                channel
                    .send_message(ctx.http(), |a| {
                        a.embed(|b| {
                            b.title(poolname)
                                .url(format!("https://arbiscan.io/tx/0x{:x}", tx))
                                .field("Bribe creator", fromadress, false)
                                .field("Token", format!("0x{:x}", erctoken), false)
                                .field("Amount", readableamount, false)
                                .footer(|f| {
                                    f.text("Sliz productions".to_string()).icon_url(
                                        "https://solidlizard.finance/images/ui/lz-logo.png",
                                    )
                                })
                                .timestamp(utc)
                        })
                    })
                    .await?;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        lastblock = currentblock;
        let mut timecount = 0;
        while timecount < 60 {
            // Stops the bot
            if STOPBOOL.load(Relaxed) {
                channel.say(ctx.http(), "The bribebot is stopped!").await?;
                poise::serenity_prelude::Context::set_activity(
                    ctx.serenity_context(),
                    Activity::watching("A stop sign"),
                )
                .await;
                break 'mainloop;
            }
            // Updates the bot
            if UPDATEBOOL.load(Relaxed) {
                UPDATEBOOL.swap(false, Relaxed);
                ctx.say("Started updating the bot").await?;
                hashmapofpools.clear();
                veccontracts.clear();
                veccontracts.push("0x98A1De08715800801E9764349F5A71cBe63F99cd".parse::<H160>()?);
                let internaltxvec = arbscanclient
                    .get_internal_transactions(InternalTxQueryOption::ByAddress(address), None)
                    .await?;
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

                        hashmapofpools.insert(*ad, name);
                        count += 1;
                        if count % 10 == 0 {
                            messagehandle
                                .edit(ctx.http(), |b| {
                                    b.content(format!(
                                        "Starting setup!\n{} Contracts indexed!",
                                        count
                                    ))
                                })
                                .await?;
                        }
                    }
                }
                messagehandle
                    .edit(ctx.http(), |b| {
                        b.content(format!("Starting setup!\n{} Contracts indexed!", count))
                    })
                    .await?;

                let response = reqwest::get(
                    "https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json",
                )
                .await?;
                let jsontoken: Logos = response.json().await?;
                token = jsontoken.tokens;

                ctx.channel_id()
                    .say(
                        ctx,
                        format!("*Found {} contracts to watch!*", veccontracts.len()),
                    )
                    .await?;
            }
            // To make the bot responsive, we loop over these if function 60 times, and thus being 5 minutes,
            // instead of simply waiting 5 minutes and then checking the statements again.
            timecount += 1;
            tokio::time::sleep_until(currenttime + tokio::time::Duration::from_secs(5 * timecount))
                .await;
        }
    }

    Ok(())
}
