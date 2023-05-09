use crate::commands::allbribes::allbribes;
use crate::commands::databasesetup::databasesetup;
use crate::{Error, DB};

use ethers::types::Address;

use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use surrealdb::sql::Thing;

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

#[derive(Debug, Deserialize, Serialize)]
struct Contact {
    userid: UserId,
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
    #[description = "Get a list of all bribes up till now"] all: Option<Visibility>,
) -> Result<(), Error> {
    // Creates a new database and fetches bribes
    if startblock.is_some() {
        let delete = delete.unwrap_or(false);

        databasesetup(ctx, delete, startblock.unwrap()).await?;
    }
    // deletes all bribes in a database without creating a new one
    if delete.is_some() && startblock.is_none() {
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

        // database entry
        let _querycreation: Contact = match DB
            .create("contact")
            .content(Contact {
                userid: ctx.author().id,
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
            Visibility::DM => {}
        };
    }
    // Search the database

    Ok(())
}
