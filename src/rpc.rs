use crate::{ZcashWalletd, info};
use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{post, State};

#[derive(Serialize, Deserialize)]
pub struct CreateAccountRequest {
    label: Option<String>,
}

#[post("/create_account", data = "<request>")]
pub async fn create_account(
    request: Json<CreateAccountRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::CreateAccountResponse>, Debug<anyhow::Error>> {
    let request = request.into_inner();
    let name = request.label.unwrap_or("".to_string());

    let rep = wallet.create_account(Some(name)).await?;

    Ok(Json(rep))
}
#[derive(Serialize, Deserialize)]
pub struct CreateAddressRequest {
    account_index: u32,
    label: Option<String>,
}

#[post("/create_address", data = "<request>")]
pub async fn create_address(
    request: Json<CreateAddressRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::CreateAddressResponse>, Debug<anyhow::Error>> {
    let request = request.into_inner();
    let name = request.label.unwrap_or("".to_string());
    
    let rep = wallet.create_address(request.account_index, Some(name)).await?;

    Ok(Json(rep))
}
#[derive(Serialize, Deserialize)]
pub struct GetAccountsRequest {
    tag: Option<String>,
}

#[post("/get_accounts", data = "<_request>")]
pub async fn get_accounts(
    _request: Json<GetAccountsRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::GetAccountsResponse>, Debug<anyhow::Error>> {
    
    let rep = wallet.get_accounts(None).await?;
    
    Ok(Json(rep))
}

#[derive(Serialize, Deserialize)]
pub struct GetTransactionByIdRequest {
    pub txid: String,
    pub account_index: u32,
}

#[post("/get_transfer_by_txid", data = "<request>")]
pub async fn get_transaction(
    request: Json<GetTransactionByIdRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::GetTransactionByIdResponse>, Debug<anyhow::Error>> {
    let request = request.into_inner();

    let rep = wallet.get_transaction(request.txid, request.account_index).await?;
    info!("{rep:?}");
    Ok(Json(rep))
}

#[derive(Serialize, Deserialize)]
pub struct GetTransfersRequest {
    pub account_index: u32,
    pub r#in: bool,
    pub subaddr_indices: Vec<u32>,
}

#[post("/get_transfers", data = "<request>")]
pub async fn get_transfers(
    request: Json<GetTransfersRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::GetTransfersResponse>, Debug<anyhow::Error>> {
    let request = request.into_inner();
    assert!(request.r#in);
    
    let rep = wallet.get_transfers(request.account_index, true, request.subaddr_indices).await?;
    Ok(Json(rep))
}

#[derive(Serialize, Deserialize)]
pub struct GetFeeEstimateRequest {}

#[post("/get_fee_estimate", data = "<_request>")]
pub fn get_fee_estimate(
    _request: Json<GetFeeEstimateRequest>,
) -> Result<Json<crate::GetFeeEstimateResponse>, Debug<anyhow::Error>> {
    let rep = crate::get_fee_estimate();

    Ok(Json(rep))
}

#[derive(Serialize, Deserialize)]
pub struct GetHeightRequest {}

#[post("/get_height", data = "<_request>")]
pub async fn get_height(
    _request: Json<GetHeightRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::GetHeightResponse>, Debug<anyhow::Error>> {
    let rep = wallet.get_height().await?;

    Ok(Json(rep))
}

#[derive(Serialize, Deserialize)]
pub struct SyncInfoRequest {}

#[post("/sync_info", data = "<_request>")]
pub async fn sync_info(
    _request: Json<SyncInfoRequest>,
    wallet: &State<ZcashWalletd>,
) -> Result<Json<crate::SyncInfoResponse>, Debug<anyhow::Error>> {
    let rep = wallet.sync_info().await?;

    Ok(Json(rep))
}

#[post("/request_scan")]
pub async fn request_scan(
    wallet: &State<ZcashWalletd>,
) -> Result<(), Debug<anyhow::Error>> {
    wallet.request_scan().await?;

    Ok(())
}
