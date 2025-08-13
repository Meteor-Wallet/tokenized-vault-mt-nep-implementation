use near_sdk::{json_types::U128, NearToken};
use near_workspaces::{Account, Contract};
use serde_json::json;

pub async fn deploy_and_init_mock_mt(
    owner: &Account,
) -> Result<Contract, Box<dyn std::error::Error>> {
    let contract_code = near_workspaces::compile_project("./mock_contracts/mock_mt").await?;

    let contract = owner.deploy(&contract_code).await?.into_result()?;

    contract.call("new").transact().await?.into_result()?;

    Ok(contract)
}

pub async fn mt_mint(
    contract: &Contract,
    account: &Account,
    token_id: &str,
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    account
        .call(contract.id(), "mint")
        .args_json(json!({
            "account_id": account.id(),
            "token_id": token_id,
            "amount": amount.to_string(),
        }))
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

// mt_transfer_call functionality moved to vault.rs to avoid duplication
// Use mt_transfer_call_deposit from vault helper instead

pub async fn mt_balance_of(
    contract: &Contract,
    account: &Account,
    token_id: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(contract.id(), "mt_balance_of")
        .args_json(json!({
            "account_id": account.id(),
            "token_id": token_id,
        }))
        .await?
        .json()?;

    Ok(result.0)
}
