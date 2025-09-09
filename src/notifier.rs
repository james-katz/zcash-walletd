// notifier.rs
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use tracing::warn;

fn txid_to_hex_le(txid_be: &[u8]) -> String {
    let mut v = txid_be.to_vec();
    v.reverse();
    hex::encode(v)
}

#[async_trait]
pub trait TxNotifier: Send + Sync + 'static {
    async fn notify_tx(&self, txid: &[u8]) -> Result<()>;
}

pub struct HttpNotifier {
    client: Client,
    base_url: String,
}

impl HttpNotifier {
    pub fn new(base_url: impl Into<String>, accept_invalid_certs: bool) -> Result<Self> {
        // TODO: Remove self signed certificate accept
        let client = Client::builder()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()?;
        Ok(Self { client, base_url: base_url.into() })
    }
}

#[async_trait]
impl TxNotifier for HttpNotifier {
    async fn notify_tx(&self, txid: &[u8]) -> Result<()> {
        let hexid = txid_to_hex_le(txid);
        let url = format!("{}{}", self.base_url, hexid);

        // Best-effort notify: warn but don't fail the pipeline
        if let Err(e) = self.client.get(url).send().await {
            warn!("Failed to notify new tx: {e}");
        }
        Ok(())
    }
}