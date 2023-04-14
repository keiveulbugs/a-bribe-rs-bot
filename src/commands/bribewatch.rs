use crate::{Error, STOPBOOL, UPDATEBOOL};
use chrono::{prelude::Utc, DateTime};
use ethers::{
    core::abi::AbiDecode,
    providers::{Http, Middleware, Provider},
    types::{Address, Chain, Filter, H160, H256, U256},
    utils::format_units,
};
use ethers_etherscan::account::InternalTxQueryOption;
use poise::serenity_prelude::{Activity, CacheHttp, ChannelId};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

const BRIBEFACTORY: &str = dotenv!("BRIBEFACTORY");
const ARBSCANKEY: &str = dotenv!("ARBSCAN");

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

// Command that starts watching all blocks for contract interaction
#[poise::command(slash_command)]
pub async fn bribewatch(
    ctx: poise::Context<'_, (), Error>,
    #[description = "Channel to post updates"] channel: ChannelId,
) -> Result<(), Error> {
    ctx.say("yihaaa").await?;
    let response =
        reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
            .await?;
    let jsontoken: Logos = response.json().await?;

    let mut token = jsontoken.tokens;

    // let provider = Provider::<Ws>::connect(format!("wss://arb-mainnet.g.alchemy.com/v2/{}", ALCHEMYKEY))
    // .await
    // .map_err(|wserr| format!("Couldn't connect to the Alchemy websocket! {}", wserr))?;
    let provider = Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?;
    let client = Arc::new(&provider);
    let mut veccontracts = vec![];
    let address: Address = BRIBEFACTORY.parse()?;
    let arbscanclient = ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;

    if UPDATEBOOL.load(Relaxed) {
        UPDATEBOOL.swap(false, Relaxed);
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
        let response =
            reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
                .await?;
        let jsontoken: Logos = response.json().await?;
        token = jsontoken.tokens;

        ctx.channel_id()
            .say(ctx, format!("*Found {} contracts to watch!*", count))
            .await?;
    }
    let mut lastblock = provider.get_block_number().await? - 1000;

    'mainloop: loop {
        if STOPBOOL.load(Relaxed) {
            channel.say(ctx.http(), "The bribebot is stopped!").await?;
            break 'mainloop;
        }
        let currenttime = tokio::time::Instant::now();
        //let timeinu64 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() + 500;
        let utc: DateTime<Utc> = Utc::now(); // + chrono::Duration::seconds(300);
        let currentblock = provider.get_block_number().await?;
        let status = format!("block {}", currentblock);
        poise::serenity_prelude::Context::set_activity(
            ctx.serenity_context(),
            Activity::watching(status),
        )
        .await;
        let filter = Filter::new()
            //.to_block(currentblock)
            .to_block(65886514)
            .from_block(65886512)
            //.from_block(lastblock)
            .topic0(
                "0xf70d5c697de7ea828df48e5c4573cb2194c659f1901f70110c52b066dcf50826"
                    .parse::<H256>()?,
            )
            .address(veccontracts.clone())
            .address("0x98A1De08715800801E9764349F5A71cBe63F99cc".parse::<H160>()?);

        let logs = client.get_logs(&filter).await?;
        println!("{} transactions found!", logs.iter().len());
        for log in logs {
            println!("test {:#?}", log);
            let erctoken = Address::from(log.topics[2]);
            let fromaddress = Address::from(log.topics[1]);
            let amount = U256::decode(log.data)?;
            let tx = log.transaction_hash.unwrap();

            let mut readableamount = format_units(amount, "ether")?;
            let splitting = readableamount.find('.').unwrap() + 3;
            readableamount.truncate(splitting);

            if let Some(tokenname) = token
                .iter()
                .find(|p| p.address.to_lowercase() == format!("0x{:x}", erctoken))
            {
                let imageurl = tokenname
                    .logo_uri
                    .clone()
                    .ok_or("https://solidlizard.finance/images/ui/lz-logo.png".to_string())?;
                channel
                    .send_message(ctx.http(), |a| {
                        a.embed(|b| {
                            b.title("New Bribe!".to_string())
                                .url(format!("https://arbiscan.io/tx/0x{:x}", tx))
                                .field("Bribe creator", format!("0x{:x}", fromaddress), false)
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
                channel
                    .send_message(ctx.http(), |a| {
                        a.embed(|b| {
                            b.title("New Bribe!".to_string())
                                .url(format!("https://arbiscan.io/tx/0x{:x}", tx))
                                .field("Bribe creator", format!("0x{:x}", fromaddress), false)
                                .field("Token", erctoken, false)
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

            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
        lastblock = currentblock;
        tokio::time::sleep_until(currenttime + tokio::time::Duration::from_secs(300)).await;
    }

    Ok(())
}
