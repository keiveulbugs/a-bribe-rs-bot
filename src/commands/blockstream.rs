use crate::{Error, STOPBOOL, UPDATEBOOL};

use ethers::{
    prelude::abigen,
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, Chain},
};
use ethers_etherscan::account::InternalTxQueryOption;
use poise::serenity_prelude::Activity;
use poise::serenity_prelude::{CacheHttp, ChannelId};
use rust_embed::RustEmbed;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use std::sync::atomic::*;
use std::sync::Arc;

const BRIBEFACTORY: &str = dotenv!("BRIBEFACTORY");
const ALCHEMYKEY: &str = dotenv!("ALCHEMY");
const ARBSCANKEY: &str = dotenv!("ARBSCAN");

#[derive(RustEmbed)]
#[folder = "src/commands/jsonfiles"]
#[prefix = "json_files/"]
pub struct Asset;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    #[serde(rename = "bribeEntities")]
    pub bribe_entities: Vec<BribeEntity>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BribeEntity {
    pub id: String,
}

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

abigen!(
    IERC20,
    r#"[
        event notifyRewardAmount(address indexed from, address indexed to, uint256 value)
    ]"#,
);

// Command that starts watching all blocks for contract interaction
#[poise::command(slash_command)]
pub async fn blockstream(
    ctx: poise::Context<'_, (), Error>,
    #[description = "Channel to post updates"] channel: ChannelId,
) -> Result<(), Error> {
    ctx.send(|b| b.content("Start creating all constants!").ephemeral(true))
        .await
        .map_err(|_| "Was not succesful in starting the command!")?;
    STOPBOOL.swap(false, Ordering::Relaxed);
    UPDATEBOOL.swap(true, Ordering::Relaxed);
    let _bribevoter = "0x98a1de08715800801e9764349f5a71cbe63f99cc".parse::<Address>()?;

    let logofile =
        Asset::get("json_files/arbi-list.json").expect("Could not open json file with logos!");
    let logostring = std::str::from_utf8(logofile.data.as_ref()).expect("Could not parse logos");
    let logojson: Logos =
        serde_json::from_str(logostring).map_err(|b| format!("Something went wrong: {}", b))?;

    let token = logojson.tokens;

    let address: Address = BRIBEFACTORY.parse()?;
    let arbscanclient = ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;
    let mut veccontracts = vec![];

    ctx.send(|c| c.content("Starting to watch the blocks!").ephemeral(true))
        .await
        .map_err(|_| "Couldn't start watching blocks!")?;

    let mut errorcount: u32 = 0;

    let ctx2 = ctx;

    'mainloop: loop {
        let provider =
            Provider::<Ws>::connect(format!("wss://arb-mainnet.g.alchemy.com/v2/{}", ALCHEMYKEY))
                .await
                .map_err(|wserr| format!("Couldn't connect to the Alchemy websocket! {}", wserr))?;
        let client = Arc::new(&provider);
        let mut streaming = provider
            .subscribe_blocks()
            .await
            .map_err(|streamerr| format!("Couldn't create a streaming connection: {}", streamerr))?
            .fuse();

        // Error counter
        errorcount += 5;
        if errorcount > 15 {
            channel.say(ctx2.http(), "The websocket seems to struggle a lot, so calling it a day and leaving the loop. Please restart the command or check if your websocket api key is not over its limit yet.").await?;
            break 'mainloop;
        }

        // Here the loop starts
        'streamloop: while let Some(block) = streaming.next().await {
            // Check if the slashcommand is canceled
            if STOPBOOL.load(Ordering::Relaxed) {
                channel.say(ctx2.http(), "The bribebot is stopped!").await?;
                break 'mainloop;
            }
            // Check if the contracts should be updated
            if UPDATEBOOL.load(Ordering::Relaxed) {
                UPDATEBOOL.swap(false, Ordering::Relaxed);
                let internaltxvec = arbscanclient
                    .get_internal_transactions(InternalTxQueryOption::ByAddress(address), None)
                    .await?;
                let mut count = 0;
                for tx in internaltxvec {
                    if tx.result_type == "create" && tx.contract_address.value().is_some() {
                        let ad = tx.contract_address.value().unwrap();
                        veccontracts.push(*ad);
                        count += 1;
                    }
                }
                channel
                    .say(ctx.http(), format!("*Found {} contracts to watch!*", count))
                    .await?;
            }
            // Reduce error count if the bot works properly to avoid one offs.
            errorcount = errorcount.saturating_sub(1);
            // Getting the blocknumber
            let numberblockresult = block.number;
            let numberblock = match numberblockresult {
                Some(val) => val,
                None => {
                    continue 'streamloop;
                }
            };
            //println!("{}", numberblock);
            // Getting the block
            let blockresultoption = &client.get_block_with_txs(numberblock).await;

            let blockoption = match blockresultoption {
                Ok(val) => val,
                Err(error) => {
                    ctx2.send(|b| {
                        b.content(format!(
                            "Couldn't fetch block {} from the rpc with error: {}",
                            numberblock, error
                        ))
                        .ephemeral(true)
                    })
                    .await?;
                    continue 'streamloop;
                }
            };
            //  println!("at 164");
            let block2 = match blockoption {
                Some(val) => val,
                None => {
                    ctx2.send(|b| {
                        b.content(format!(
                            "Block {} doesn't seem to have any content.",
                            numberblock
                        ))
                        .ephemeral(true)
                    })
                    .await?;
                    continue 'streamloop;
                }
            };

            // Staying alive
            if numberblock % 1000 == 0.into() {
                // channel
                //     .say(ctx2.http(), format!("At block = {}", numberblock))
                //     .await?;
                let status = format!("block {}", numberblock);
                poise::serenity_prelude::Context::set_activity(
                    ctx2.serenity_context(),
                    Activity::watching(status),
                )
                .await;

                // This restarts the websocket if there is too much of a delay
                let tempblock = provider.get_block_number().await?;
                if tempblock > (numberblock + 50) {
                    continue 'mainloop;
                }
            }
            // println!("{}", numberblock);
            let vectx101 = &block2.transactions;
            for tx in vectx101 {
                if tx.to.is_some() {
                    // This unwrap should be safe as we check above that the tx.to is some()
                    for receiver in tx.to {
                        if veccontracts.contains(&receiver) {
                            // let txval :Address = "0x9175fa90bea50873e004f42f4cf1e27ad3b5e64f34f8d74400ca34411d629710".parse()?;
                            // let fed = arbscanclient.get_transactions(&txval, None).await?;
                            // for ts in fed {
                            //     let functionname = ts.function_name.unwrap();
                            //     println!("{}", functionname);
                            // }
                            // let trace =  provider.trace_transaction(tx.hash).await?;
                            // for t in trace {
                            //     println!("{:#?}", t);
                            // }

                            // channel
                            //     .say(ctx2.http(), format!("Interaction: {:?}", receiver))
                            //     .await?;
                            let _input = tx.clone().input;
                            let _briber = tx.from;

                            let mut url =
                                "https://solidlizard.finance/images/ui/lz-logo.png".to_string();
                            if let Some(logouri) =
                                token.iter().find(|p| p.symbol.to_lowercase() == "aopenx")
                            {
                                if logouri.logo_uri.is_some() {
                                    url = logouri.logo_uri.clone().unwrap();
                                }
                            };

                            channel
                                    .send_message(ctx2.http(), |b| {
                                        b.embed(|b| {
                                            b.title("Transaction on Arbiscan")
                                                .url(format!("https://arbiscan.io/tx/0x{:x}", tx.hash))
                                                .field("Pool", format!("> 0x{:x}", receiver), false)
                                                .field("From", format!("> 0x{:x}", tx.from), false)
                                                .timestamp(ctx.created_at())
                                               // .image(url.to_string())
                                                // .thumbnail(url.to_string())
                                                // .thumbnail(url.to_string())
                                                .footer(|f| {
                                                    f.text("Sliz productions".to_string()).icon_url(
                                                        "https://solidlizard.finance/images/ui/lz-logo.png",
                                                    )
                                                })
                                        })
                                    })
                                    .await?;
                        }
                        //                         if receiver == bribevoter {
                        //                             let filterbribe = Filter::new()
                        //                                 .address(bribevoter)
                        //                                 .at_block_hash(block.hash.unwrap())
                        //                                 .event("notifyRewardAmount(address indexed from, address indexed to, uint256 value)")
                        //                                 .event("distribute()")
                        //                                 .event("distributeAll()");
                        //                             let logs = client.get_logs(&filterbribe).await?;

                        //                             channel
                        //                                     .send_message(ctx2.http(), |b| {
                        //                                         b.embed(|b| {
                        //                                             b.title("Transaction on Arbiscan")
                        //                                                 .url(format!("https://arbiscan.io/tx/0x{:x}", tx.hash))
                        //                                                 .description("A Bribe from the Solid Lizzard Finance voter appeared.")
                        //                                                 .field("Pool", format!("> {:x}", receiver), false)
                        //                                                 .field("From", format!("> {:x}", tx.from), false)
                        //                                                 .timestamp(ctx.created_at())
                        //                                                // .image(url.to_string())
                        //  //                                               .thumbnail(url.to_string())
                        //                                                 // .thumbnail(url.to_string())
                        //                                                 .footer(|f| {
                        //                                                     f.text(format!("Sliz productions")).icon_url(
                        //                                                         "https://solidlizard.finance/images/ui/lz-logo.png",
                        //                                                     )
                        //                                                 })
                        //                                         })
                        //                                     })
                        //                                     .await?;
                        //                         }
                    }
                }
            }
        }
    }

    // drop(task);

    // ctx.say("ending").await?;
    Ok(())
}

// if let Some(logouri) = token.iter().find(|p| p.name == name) {
//     println!("Found person with name {}: {:?}", name, logouri);
// } else {
//     println!("No person found with name");
// }
