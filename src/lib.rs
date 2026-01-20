#[path = "generated/cash.z.wallet.sdk.rpc.rs"]
pub mod lwd_rpc;

mod account;
mod db;
pub mod monitor;
mod network;
mod notifier;
pub mod rpc;
mod scan;
pub mod transaction;

use std::{path::Path, sync::Arc};
use anyhow::{anyhow, Result};
use figment::{providers::{Env, Format, Json, Serialized}, Figment};
use rocket::{Build, Rocket};
use serde::{Serialize, Deserialize};
use tonic::{transport::Channel, Request};
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
    EnvFilter, Layer, Registry,
};
use crate::{account::{AccountBalance, SubAccount}, db::Db, lwd_rpc::{compact_tx_streamer_client::CompactTxStreamerClient, Empty}, monitor::MonitorTask, network::Network, notifier::{HttpNotifier, TxNotifier}, scan::{get_latest_height, Decoder, Orchard, Sapling, ScanError}, transaction::Transfer};
use zcash_client_backend::keys::UnifiedFullViewingKey;

pub type Hash = [u8; 32];
pub type Client = CompactTxStreamerClient<Channel>;

#[derive(Deserialize, Debug)]
pub struct WalletConfig {
    pub port: Option<u16>,
    pub db_path: String,
    pub confirmations: u32,
    pub lwd_url: String,
    pub notify_tx_url: String,
    pub poll_interval: u16,
    pub regtest: bool,
    pub orchard: bool,
    pub vk: String,
    pub birth_height: u32,
}

impl WalletConfig {
    pub fn network(&self) -> Network {
        if self.regtest {
            Network::Regtest
        } else {
            Network::Main
        }
    }
}
#[derive(Clone)]
pub struct ZcashWalletd {
    db: Arc<Db>,
    pub config: Arc<WalletConfig>,
}

impl ZcashWalletd {
    pub async fn init(rocket: Option<&Rocket<Build>>) -> anyhow::Result<Self> {
        dotenv::dotenv().ok();
        // env_logger::init();
        let config_path = dotenv::var("CONFIG_PATH")
            .ok()
            .unwrap_or("/data/config.json".to_string());
        let _ = Registry::default()
            .with(default_layer())
            .with(env_layer())
            .try_init();
        
        let mut figment = match rocket {
        Some(r) => r.figment().clone(),
        None => Figment::new()
            .merge(Serialized::default("port", None::<u16>)),
        };

        figment = figment.merge(Env::raw());
        info!("figment {figment:?}");
        let config = Path::new(&config_path);
        if config.exists() {
            figment = figment.merge(Json::file(config_path));
        }

        let config: WalletConfig = figment.extract().unwrap();
        info!("Config {config:?}");
        let network = config.network();
        assert!(config.orchard);

        let ufvk = &config.vk;
        let birth_height = config.birth_height;
        let ufvk = UnifiedFullViewingKey::decode(&network, ufvk)
            .map_err(|_| anyhow!("Invalid Unified Viewing Key"))?;
        
        let tx_notifier = match rocket {
            Some(_) => {
                let http = HttpNotifier::new(config.notify_tx_url.clone(), true)?;
                Some(Arc::new(http) as Arc<dyn TxNotifier>)
            }
            None => None,
        };
        let db = Db::new(network, &config.db_path, &ufvk, tx_notifier).await?;
        let db_exists = db.create().await?;
        if !db_exists {
            db.new_account("").await?;
        }
        let mut client = CompactTxStreamerClient::connect(config.lwd_url.clone()).await?;
        db.fetch_block_hash(&mut client, birth_height).await?;

        Ok (
            Self {
                db: Arc::new(db),
                config: Arc::new(config),
            }
        )
    }

    pub fn monitor_task(&self) {
        let poll_secs = self.config.poll_interval as u64;
        let _handle = MonitorTask::spawn(self.clone(), poll_secs);
    }

    pub async fn create_account(&self, label: Option<String>) -> anyhow::Result<CreateAccountResponse> {
        let name = label.unwrap_or("".to_string());
        let account = self.db.new_account(&name).await?;
        
        Ok(
            CreateAccountResponse {
                account_index: account.account_index,
                address: account.address
            }
        )
    }

    pub async fn create_address(
        &self,
        account_index: u32,
        label: Option<String>
    ) -> anyhow::Result<CreateAddressResponse> {
        let name = label.unwrap_or("".to_string());
        let sub_account = self.db.new_sub_account(account_index, &name).await?;

        Ok(
            CreateAddressResponse {
                address: sub_account.address.clone(),
                address_index: sub_account.sub_account_index
            }
        )
    }

    pub async fn get_accounts(&self, _tag: Option<String>) -> anyhow::Result<GetAccountsResponse> {
        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(from_tonic)?;
        let latest_height = get_latest_height(&mut client).await?;
        let sub_accounts = self.db.get_accounts(latest_height, self.config.confirmations).await?;
        let total_balance: u64 = sub_accounts.iter().map(|sa| sa.balance).sum();
        let total_unlocked_balance: u64 = sub_accounts.iter().map(|sa| sa.unlocked_balance).sum();

        Ok(
            GetAccountsResponse {
                subaddress_accounts: sub_accounts,
                total_balance,
                total_unlocked_balance,
            }
        )    
    }

    pub async fn get_addresses(&self) -> anyhow::Result<GetAddressesResponse> {
        let addresses = self.db.get_addresses().await?;

        Ok(
            GetAddressesResponse {
                addresses
            }
        )
    }

    pub async fn get_transaction(
        &self,
        txid: String,
        account_index: u32
    ) -> anyhow::Result<GetTransactionByIdResponse>{
        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(from_tonic)?;
        let latest_height = get_latest_height(&mut client).await?;
        let transfers = self.db
            .get_transfers_by_txid(
                latest_height,
                &txid,
                account_index,
                self.config.confirmations,
            )
            .await?;
        
        Ok(
            GetTransactionByIdResponse {
                transfer: transfers[0].clone(),
                transfers,
            }
        )
    }

    pub async fn get_transfers(
        &self,
        account_index: u32,
        r#in: bool,
        subaddr_indices: Vec<u32>
    ) -> anyhow::Result<GetTransfersResponse> {
        assert!(r#in);
        
        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(from_tonic)?;
        let latest_height = get_latest_height(&mut client).await?;
        let transfers = self.db
            .get_transfers(
                latest_height,
                account_index,
                &subaddr_indices,
                self.config.confirmations,
            )
            .await?;
        Ok(
            GetTransfersResponse { r#in: transfers }
        )
    }

    pub async fn get_height(&self) -> anyhow::Result<GetHeightResponse> {
        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(from_tonic)?;
        let latest_height = get_latest_height(&mut client).await?;
        Ok(
            GetHeightResponse {
                height: latest_height,
            }
        )
    }

    pub async fn get_wallet_height(&self) -> anyhow::Result<GetHeightResponse> {
        let synced_height = self.db.get_synced_height().await?;
        Ok(
            GetHeightResponse {
                height: synced_height,
            }
        )
    }

    pub async fn sync_info(&self) -> anyhow::Result<SyncInfoResponse> {
        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(from_tonic)?;
        let rep = client
            .get_lightd_info(Request::new(Empty {}))
            .await
            .map_err(from_tonic)?
            .into_inner();
        Ok(
            SyncInfoResponse {
                target_height: rep.block_height as u32,
                height: rep.estimated_height as u32,
            }
        )
    }

    pub async fn request_scan(&self) -> anyhow::Result<()> {
        let network = self.config.network();
        let ufvk = self.db.ufvk();
        let start = self.db.get_synced_height().await?;
        let prev_hash = self.db
            .get_block_hash(start)
            .await?
            .ok_or(anyhow::anyhow!("Block Hash missing from db"))?;

        let nfs = self.db.get_nfs().await?;
        let mut sap_dec = ufvk.sapling().map(|fvk| {
            let nk = fvk.fvk().vk.nk;
            let ivk = fvk.to_ivk(zip32::Scope::External);
            let pivk = sapling_crypto::keys::PreparedIncomingViewingKey::new(&ivk);
            // TODO: Load nfs
            Decoder::<Sapling>::new(nk, fvk.clone(), pivk, &nfs)
        });
        let mut orc_dec = ufvk.orchard().map(|fvk| {
            let ivk = fvk.to_ivk(zip32::Scope::External);
            let pivk = orchard::keys::PreparedIncomingViewingKey::new(&ivk);
            // TODO: Load nfs
            Decoder::<Orchard>::new(fvk.clone(), ivk, pivk, &nfs)
        });

        let mut client = CompactTxStreamerClient::connect(self.config.lwd_url.clone())
            .await
            .map_err(anyhow::Error::new)?;
        let end = get_latest_height(&mut client).await?;
        
        info!("Scan from {start} to {end}");
        if start >= end {
            return Ok(());
        }

        let res = crate::scan::scan(
            &network,
            &mut client,
            start + 1,
            end,
            &prev_hash,
            &mut sap_dec,
            &mut orc_dec,
        )
        .await;
        match res {
            Err(error) =>
            // Rewind if we hit a chain reorg but don't error
            {
                match error {
                    ScanError::Reorganization => {
                        let synced_height = self.db.get_synced_height().await?;
                        self.db.truncate_height(synced_height - SAFE_REORG_DISTANCE)
                            .await
                    }
                    ScanError::Other(error) => Err(error),
                }?
            }

            Ok(events) => {
                self.db.store_events(&events).await?;
            }
        }
        Ok(())
    }
}

pub const SAFE_REORG_DISTANCE: u32 = 100u32;

#[derive(Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub account_index: u32,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAddressResponse {
    pub address: String,
    pub address_index: u32,
}

#[derive(Serialize, Deserialize)]
pub struct GetAccountsResponse {
    pub subaddress_accounts: Vec<AccountBalance>,
    pub total_balance: u64,
    pub total_unlocked_balance: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetAddressesResponse {
    pub addresses: Vec<SubAccount>,    
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetTransactionByIdResponse {
    pub transfer: Transfer,
    pub transfers: Vec<Transfer>,
}

#[derive(Serialize, Deserialize)]
pub struct GetTransfersResponse {
    pub r#in: Vec<Transfer>,
}

#[derive(Serialize, Deserialize)]
pub struct GetFeeEstimateResponse {
    pub fee: u64,
}

// Roughly estimate at 2 transparent in/out + 2 shielded in/out
// We cannot implement ZIP-321 here because we don't have
// the transaction
const LOGICAL_ACTION_FEE: u64 = 5000u64;

pub fn get_fee_estimate() -> GetFeeEstimateResponse {
    GetFeeEstimateResponse {
        fee: 4 * LOGICAL_ACTION_FEE,
    }
}

#[derive(Serialize, Deserialize)]
pub struct GetHeightResponse {
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct SyncInfoResponse {
    pub target_height: u32,
    pub height: u32,
}

pub async fn notify_tx(txid: &[u8], notify_tx_url: &str) -> Result<()> {
    let mut txid = txid.to_vec();
    txid.reverse();
    let txid = hex::encode(&txid);
    info!("Notify tx {}", &txid);

    let url = notify_tx_url.to_string() + &txid;
    // TODO: Remove self signed certificate accept
    let res = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?
        .get(url)
        .send()
        .await;
    if let Err(e) = res {
        log::warn!("Failed to notify new tx: {e}",);
    }

    Ok(())
}

#[allow(dead_code)]
fn to_tonic<E: ToString>(e: E) -> tonic::Status {
    tonic::Status::internal(e.to_string())
}

fn from_tonic<E: ToString>(e: E) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync + 'static>;

fn default_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fmt::layer()
        .with_ansi(false)
        .with_span_events(FmtSpan::ACTIVE)
        .compact()
        .boxed()
}

fn env_layer<S>() -> BoxedLayer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .boxed()
}
