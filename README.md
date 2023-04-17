# Bribe watching bot

**Make sure to install the Rust toolchain and Cargo!**

https://www.rust-lang.org/learn/get-started

Add your environment variables in the .env.

Then simply run `cargo build --release` and you will get an executable. Run it, and it should be working.

*Good to know*
When using the slash command, it checks the last 1000 blocks to see if there are bribes you missed.
You can change this in the bribewatch.rs file around line 165.
After checking the last 1000 blocks, it will only check new blocks.


**ToDo**
- [ ] Create a database system so that users can add their address to their username
- [ ] Allow with slash commands to fetch how many bribes a wallet made