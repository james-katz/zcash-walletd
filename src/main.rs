#[macro_use]
extern crate rocket;

use anyhow::Result;

use clap::Parser;
use zcash_walletd::{rpc::*, ZcashWalletd};

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(short, long)]
    rescan: bool,
}

// They come from the config file
//
// const DB_PATH: &str = "zec-wallet.db";
// const CONFIRMATIONS: u32 = 10;
//
// pub const LWD_URL: &str = "https://lite.ycash.xyz:9067";
// pub const NOTIFY_TX_URL: &str = "https://localhost:14142/zcashlikedaemoncallback/tx?cryptoCode=yec&hash=";

#[rocket::main]
async fn main() -> Result<()> {
    let rocket = rocket::build();
    let wallet = ZcashWalletd::init(Some(&rocket)).await?;
    wallet.monitor_task();
    
    rocket
        .manage(wallet)
        .mount(
            "/",
            routes![
                create_account,
                create_address,
                get_accounts,
                get_transaction,
                get_transfers,
                get_fee_estimate,
                get_height,
                sync_info,
                request_scan,
            ],
        )
        .launch()
        .await?;

    Ok(())
}