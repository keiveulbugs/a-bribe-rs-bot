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
use poise::serenity_prelude::ButtonStyle;
use poise::serenity_prelude::CollectComponentInteraction;

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
pub async fn total_bribes(
    ctx: poise::Context<'_, (), Error>,
   // #[description = "Contract to receive total amount of bribes of"] contractinput: String,
    #[description = "Since which block"]    blockinput: u64,
) -> Result<(), Error> {
    
    let firstmessage = ctx.send(|b| {b.embed(|c| {c.description("Starting to look up the contract")})}.ephemeral(true)).await?;

    let response =
        reqwest::get("https://raw.githubusercontent.com/DecentST/arblist/main/arbi-list.json")
            .await?;
    let jsontoken: Logos = response.json().await?;

    let token = jsontoken.tokens;

    let provider = Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?;
    let client = Arc::new(&provider);

    let arbscanclient = ethers_etherscan::Client::new(Chain::Arbitrum, ARBSCANKEY)?;
    let mut veccontracts = vec!["0x98A1De08715800801E9764349F5A71cBe63F99cc".parse::<H160>()?];
    let address: Address = BRIBEFACTORY.parse()?;
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
            count += 1;
            //println!("{}", count);
        }
    }


    // let pool :Address = match contractinput.parse::<H160>() {
    //     Ok(val) => val,
    //     Err(_) => {
    //         firstmessage.edit(ctx, |b| {b.content("This doesn't look like a valid address").ephemeral(true)}).await?;
    //         return Ok(());
    //     }
    // };

   // let contract = PoolContract::new(pool, client.clone());

    let currentblock = match provider.get_block_number().await {
        Ok(val) => val,
        Err(_) => {                
            match provider.get_block_number().await {
                Ok(val) => val,
                Err(_) => {
                    firstmessage.edit(ctx, |b| {b.content("Couldn't get the latest block").ephemeral(true)}).await?;
                    return Ok(());
                }
            }
        }
    };

    let filter = Filter::new()
    .to_block(currentblock)
    .from_block(blockinput)
    .topic0(
        "0xf70d5c697de7ea828df48e5c4573cb2194c659f1901f70110c52b066dcf50826"
            .parse::<H256>()?,
    )
    .address(veccontracts);
   // println!("after filter");

    let mut bribevec = vec![];

    let logs = client.get_logs(&filter).await?;
       // println!("{} transactions found!", logs.iter().len());
        'logs: for log in logs {
            let erctoken = Address::from(log.topics[2]);

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

            // let blockresult = match provider.get_block(logblocknumber).await {
            //     Ok(val) => val,
            //     Err(_) => {
            //         continue 'logs;
            //     }
            // };



            if let Some(tokenname) = token
                .iter()
                .find(|p| p.address.to_lowercase() == format!("0x{:x}", erctoken))
            {

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

               // bribehashmap.insert(tokenname.name.clone(), readableamount);

                let tempbribe = (tokenname.name.clone(), readableamount, false);
                bribevec.push(tempbribe);

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
             //   bribehashmap.insert("Unknown".to_string(), readableamount);
                let tempbribe = ("Unknown".to_string(), readableamount, false);
                bribevec.push(tempbribe);


            }


        }
        
      //  println!("{:#?}", bribevec);

        if bribevec.len() < 25 {
            firstmessage.edit(ctx, |b| {b.embed(|c| {c.fields(bribevec)})}).await?;
        } else {
            let mut tempvec = bribevec.chunks(25);
            let tempvec2 = tempvec.next().unwrap();
            let mut tempvec3 = vec![];
            for i in tempvec2 {
                tempvec3.push(i.clone());
            };


            firstmessage.edit(ctx, |b| {b.embed(|c| {c.fields(tempvec3)})
        .components(|a| {a.create_action_row(|f| f.create_button(|d| {
            d.custom_id("more").label("View more!").style(ButtonStyle::Primary)
        }))})}).await?;

        while let Some(mci) = CollectComponentInteraction::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(600)) //timeout after 10 minutes
        .filter(move |mci| mci.data.custom_id == "more".to_string())
        .await
    {
        for chunkss in tempvec.clone() {
            let mut chunk1 = vec![];
            for i in chunkss {
                chunk1.push(i.clone());
            }
            ctx.send(|b| {b.embed(|b| {b.fields(chunk1)})}.ephemeral(true)).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

    }


        };

        

    

    Ok(())
}
