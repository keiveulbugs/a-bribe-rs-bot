use std::vec;
use crate::{Error, STOPBOOL, UPDATEBOOL};
use ethabi::ethereum_types::H256;
use ethers::utils::format_units;
use poise::serenity_prelude::{self as serenit, ChannelId};
use serenity::utils::Colour;
use reqwest;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use image::{DynamicImage, GenericImageView, ImageBuffer, imageops, io::Reader as ImageReader};
use std::io::Cursor;
use std::{num, sync::Arc};
use ethers::prelude::Multicall;

use ethers::types::H160;
use ethers::{
    core::abi::AbiDecode,
    prelude::{abigen, Abigen},
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, BlockNumber, Chain, Filter, U256, I256},
};
use ethers_etherscan::account::InternalTxQueryOption;
use std::sync::atomic::Ordering::Relaxed;


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logos {
    pub tokens: Vec<Token>,
}
const BRIBEFACTORY: &str = dotenv!("BRIBEFACTORY");

const ARBSCANKEY: &str = dotenv!("ARBSCAN");
abigen!(
    IERC20,
    r#"[
        event notifyRewardAmount(address token, uint256 reward)
    ]"#,
);

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
const ALCHEMYKEY: &str = dotenv!("ALCHEMY");

/// About command
#[poise::command(slash_command)]
pub async fn getjson(ctx: poise::Context<'_, (), Error>) -> Result<(), Error> {
    ctx.say("hello").await?;

    // let response = reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json").await?;
    // let test :Logos = response.json().await?;
    // println!("{:#?}", test);
    // let mut namvec = vec![];

    // for nam in test.tokens {
    //     namvec.push(nam.symbol);
    // }
    // namvec.sort();

    let provider = Provider::<Ws>::connect(format!("wss://arb-mainnet.g.alchemy.com/v2/{}", ALCHEMYKEY))
    .await
    .map_err(|wserr| format!("Couldn't connect to the Alchemy websocket! {}", wserr))?;
    let client = Arc::new(&provider);
    let mut veccontracts = vec![];
    let address: Address = BRIBEFACTORY.parse()?;
    let arbscanclient =
        ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;

    if UPDATEBOOL.load(Relaxed) {
        UPDATEBOOL.swap(false, Relaxed);
        let internaltxvec = arbscanclient
            .get_internal_transactions(InternalTxQueryOption::ByAddress(address), None)
            .await?;
        let mut count = 0;
        for tx in internaltxvec {
            if tx.result_type == "create" && tx.contract_address.value().is_some() {
                let ad = tx.contract_address.value().unwrap();
                veccontracts.push(ad.clone());
                count += 1;
            }
        }
        ctx.channel_id()
            .say(ctx, format!("*Found {} contracts to watch!*", count))
            .await?;
    }

    let filter = Filter::new()
        .from_block(60872628)
        .to_block(60872630)
        .topic0("0xf70d5c697de7ea828df48e5c4573cb2194c659f1901f70110c52b066dcf50826".parse::<H256>()?)
        .address("0x90B2C589860B61D6e42A17478674bf9be04B622d".parse::<H160>()?);

        let logs = client.get_logs(&filter).await?;
        println!("{} pools found!", logs.iter().len());
        for log in logs {
            println!("test {:#?}", log);
            let erctoken = Address::from(log.topics[2]);
            let fromaddress = Address::from(log.topics[1]);
            let amount = U256::decode(log.data)?;
            
            let mut readableamount = format_units(amount, "ether")?;
            let splitting = readableamount.find(".").unwrap() + 3;
            readableamount.truncate(splitting);


            println!("********************\ntoken : 0x{:x}\naddress : 0x{:x}\n amount : {}", erctoken, fromaddress, readableamount);
            
        }


    println!("test complete");

    Ok(())
}