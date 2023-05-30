use std::iter::Sum;
use std::{dbg, println};

use crate::commands::allbribes::allbribes;
use crate::commands::databasesetup::databasesetup;
use crate::{Error, DB};

use std::borrow::Cow;
use bigdecimal::BigDecimal;

use ethers::types::Address;

use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use surrealdb::sql::Thing;

#[derive(Debug, Deserialize)]
struct Sumup {
    sum: BigDecimal,
    poolname: String,
    tokenname: String,   
}



#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

#[derive(Debug, Deserialize, Serialize)]
struct Contact {
    userid: String,
    address: Address,
}

// Struct for determining visibility of message later on.
#[derive(poise::ChoiceParameter, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Visibility {
    Private,
    Public,
    DM,
}


// Database setup command
#[poise::command(slash_command)]
pub async fn database(
    ctx: poise::Context<'_, (), Error>,
    #[description = "The first block of the database, creates the database"] startblock: Option<
        u64,
    >,
    #[description = "Delete bribe record database (before creating a new one)"] delete: Option<
        bool,
    >,
    #[description = "Add your address to addressbook"] address: Option<String>,
    #[description = "Use a custom name instead of your Discord name"] customname: Option<String>,
    #[description = "Get a list of all bribes up till now"] all: Option<Visibility>,
    #[description = "Perform a custom search"] search: Option<bool>,
) -> Result<(), Error> {
    // Creates a new database and fetches bribes
    if startblock.is_some() {
        let delete = delete.unwrap_or(false);

        databasesetup(ctx, delete, startblock.unwrap()).await?;
    }
    // deletes all bribes in a database without creating a new one
    if delete.is_some() && delete.unwrap()==true && startblock.is_none() {
        let rolesofuser = ctx.author_member().await.unwrap().permissions;
        if !rolesofuser.unwrap().administrator()
            && ctx.author().id != UserId(397118394714816513)
            && ctx.author().id != UserId(320292370161598465)
        {
            ctx.say("You don't have enough rights to do this!").await?;
            return Ok(());
        }

        // Deletes the database when requested by the user
        let deletebribe: Vec<Record> = DB.delete("bribe").await?;
        ctx.send(|b| {
            b.content(format!("Deleted {} data entries", deletebribe.len()))
                .ephemeral(true)
        })
        .await?;
    }
    // add an address to the addressbook
    if address.is_some() {
        let addressclean = address.unwrap();
        let address = match addressclean.parse::<Address>() {
            Ok(val) => val,
            Err(_) => {
                ctx.send(|b| {
                    { b.content(format!("This is not a valid address: *{}*", addressclean)) }
                        .ephemeral(true)
                })
                .await?;
                return Ok(());
            }
        };
        let username = if customname.is_some() {
            customname.unwrap()
        } else {
            format!("<@{}>", ctx.author().id)
        };

        // database entry
        let _querycreation: Contact = match DB
            .create("contact")
            .content(Contact {
                userid: username,
                address,
            })
            .await
        {
            Ok(val) => {
                ctx.send(|b| {
                    { b.content(format!("Succesfully added: *{}*", addressclean)) }.ephemeral(true)
                })
                .await?;
                val
            }
            Err(_) => {
                ctx.send(|b| {
                    { b.content(format!("Could not add address: *{}*", addressclean)) }
                        .ephemeral(true)
                })
                .await?;
                return Ok(());
            }
        };
        //dbg!(_querycreation);

        return Ok(());
    }
    // Get total list of bribes
    if all.is_some() {
        match all.unwrap() {
            Visibility::Public => {
                allbribes(ctx, false).await?;
            }
            Visibility::Private => {
                allbribes(ctx, true).await?;
            }
            Visibility::DM => {
                crate::commands::allbribes::dmallbribes(ctx).await?;
            }
        };
    }
    // Search the database
    if search.is_some() {
        /*
        pub struct Bribe {
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
         */
        let fromaddress = "0x5318f07a3a20a2f8bb0ddf14f1dd58c517a76508".parse::<ethers::types::H160 >()?;
        //let mut result= DB.query("SELECT math::sum(amount), poolname, tokenname FROM bribe GROUP BY tokenname, poolname").await?;
        ctx.say("hellooo").await?;
        let mut result= DB.query("select userid from contact where address=$currentaddress").bind(("currentaddress", fromaddress)).await?;
        let cleanresult :Vec<String> = result.take((0, "userid")).unwrap();


        dbg!(&result);
        dbg!(cleanresult);

        //let resultclean :Vec<Sumup> = result.take(0)?;



       // println!("wut {:#?}", resultclean);
       // println!("wut {:#?}", result.);

    }
    

    Ok(())
}
//note: use the `cargo:rustc-link-lib` directive to specify the native libraries to link with Cargo (see https://doc.rust-lang.org/cargo/reference/build-scripts.html#cargorustc-link-libkindname