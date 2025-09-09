use std::time::Duration;
use tokio::{task::JoinHandle, time};

use crate::ZcashWalletd;

pub struct MonitorTask;

impl MonitorTask {
    pub fn spawn(wallet: ZcashWalletd, poll_secs: u64) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(poll_secs));
            loop {
                if let Err(e) = wallet.request_scan().await {
                    log::warn!("request_scan failed: {e:#}");
                }
                interval.tick().await;
            }
        })
    }
}