use node_bindgen::derive::node_bindgen;

use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use tokio::runtime::Runtime;
use zcash_walletd::{self, transaction::Transfer, ZcashWalletd};

lazy_static! {
    pub static ref RT: Runtime = tokio::runtime::Runtime::new().unwrap();

    static ref WALLET: RwLock<Option<Arc<ZcashWalletd>>> = RwLock::new(None);
}
fn store_client(wallet: ZcashWalletd) {
    *WALLET.write().unwrap() = Some(Arc::new(wallet));
}

fn run_blocking<F, Fut, T>(f: F) -> Result<T, String>
where
    F: FnOnce(Arc<ZcashWalletd>) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let wallet = {
        let guard = WALLET.read().map_err(|_| "Wallet lock poisoned".to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Error: ZcashWalletd is not initialized".to_string())?
    };

    // Drive the future to completion on the global runtime
    RT.block_on(f(wallet))
}

#[node_bindgen]
fn init() -> Result<String, String> {
    let rep = RT.block_on(async move {
        ZcashWalletd::init(None).await        
    }).map_err(|e| e.to_string())?;
    
    store_client(rep);

    Ok("ZcashWalletd initialized.".to_string())
}

#[node_bindgen]
fn create_account(label: Option<String>) -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.create_account(label).await.map_err(|e| e.to_string())?;
    
        Ok(json::object! {
            "account_index" => rep.account_index,
            "address" => rep.address
        }.pretty(2))
    })
}

#[node_bindgen]
fn create_address(account_index: u32, label: Option<String>) -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.create_address(account_index, label).await.map_err(|e| e.to_string())?;
    
        Ok(json::object! {
            "address" => rep.address,
            "address_index" => rep.address_index
        }.pretty(2))
    })
}

#[node_bindgen]
fn get_accounts() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_accounts(None).await.map_err(|e| e.to_string())?;

        let accounts: Vec<json::JsonValue> = rep.subaddress_accounts.into_iter().map(|a| {
            json::object! {
                account_index: a.account_index,
                balance: a.balance,
                base_address: a.base_address,
                label: a.label,
                tag: a.tag,
                unlocked_balance: a.unlocked_balance,
            }
        }).collect();

        Ok(json::object! {
            subaddress_accounts: accounts,
            total_balance: rep.total_balance,
            total_unlocked_balance: rep.total_unlocked_balance,
        }.pretty(2))
    })
}

#[node_bindgen]
fn get_addresses() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_addresses().await.map_err(|e| e.to_string())?;

        let addresses: Vec<json::JsonValue> = rep.addresses.into_iter().map(|a| {
            json::object! {
                account_index: a.account_index,
                sub_address_index: a.sub_account_index,
                address: a.address
            }
        }).collect();

        Ok(json::JsonValue::from(addresses).pretty(2))
    })
}

#[node_bindgen]
fn get_transaction(txid: String, account_index: u32) -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_transaction(txid, account_index).await.map_err(|e| e.to_string())?;
        
        let transfers: Vec<json::JsonValue> = rep 
            .transfers
            .into_iter()
            .map(|t| {
                transfer_to_obj(t)
            })
            .collect();

        Ok(json::object! {
            "transfer" => transfer_to_obj(rep.transfer),
            "transfers" => transfers
        }.pretty(2))
    })
}

#[node_bindgen]
fn get_transfers(account_index: u32, subaddr_indices: Vec<u32>) -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_transfers(account_index, subaddr_indices).await.map_err(|e| e.to_string())?;

        let transfers: Vec<json::JsonValue> = rep
            .r#in
            .into_iter()
            .map(|t| {
                transfer_to_obj(t)
            })
            .collect();

        Ok(json::object! {
            "in" => transfers
        }.pretty(2))        
    })
}

#[node_bindgen]
fn get_fee_estimate() -> String {
    let rep = zcash_walletd::get_fee_estimate();
    json::object! {"fee" => rep.fee}.pretty(2)
}

#[node_bindgen]
fn get_height() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_height().await.map_err(|e| e.to_string())?;
        Ok(json::object! {"height" => rep.height}.pretty(2))
    })
}

#[node_bindgen]
fn get_wallet_height() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.get_wallet_height().await.map_err(|e| e.to_string())?;
        Ok(json::object! {"height" => rep.height}.pretty(2))
    })
}

#[node_bindgen]
fn sync_info() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let rep = wallet.sync_info().await.map_err(|e| e.to_string())?;

        Ok(json::object! {
            "target_height" => rep.target_height,
            "height" => rep.height,
        }.pretty(2))
    })
}

#[node_bindgen]
fn request_scan() -> Result<String, String> {
    run_blocking(|wallet| async move {
        let _ = wallet.request_scan().await.map_err(|e| e.to_string())?;

        Ok("request_scan done.".to_string())
    })
}

#[node_bindgen]
async fn request_scan_async() -> Result<String, String> {
    let wallet = {
        let guard = WALLET.write().map_err(|_| "Wallet lock poisoned".to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Error: ZcashWalletd is not initialized".to_string())?
    };

    RT.spawn(async move {
        let _ = wallet.request_scan().await;
        println!("request_scan_async done");
    });

    Ok("Sync task launched".to_string())
}

fn transfer_to_obj(t: Transfer) -> json::JsonValue {
    json::object! {
        "address" => t.address,
        "amount" => t.amount,
        "confirmations" => t.confirmations,
        "height" => t.height,
        "fee" => t.fee,
        "note" => t.note,
        "payment_id" => t.payment_id,
        "subaddr_index" => json::object! {"major" => t.subaddr_index.major, "minor" => t.subaddr_index.minor},
        "suggested_confirmations_threshold" => t.suggested_confirmations_threshold,
        "timestamp" => t.timestamp,
        "txid" => t.txid,
        "type" => t.r#type,
        "unlock_time" => t.unlock_time,
    }
}