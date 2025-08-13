use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{json_types::U128, NearToken};
use near_workspaces::{Account, Contract};
use serde_json::json;

pub async fn deploy_and_init_vault(
    owner: &Account,
    asset_contract: &Contract,
    asset_token_id: &str,
    vault_name: &str,
    vault_symbol: &str,
) -> Result<Contract, Box<dyn std::error::Error>> {
    let contract_code = near_workspaces::compile_project("./").await?;

    // Create a unique account for each vault deployment with sufficient balance
    let vault_id = format!(
        "v{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    let vault_account = owner
        .create_subaccount(&vault_id)
        .initial_balance(near_workspaces::types::NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    let contract = vault_account.deploy(&contract_code).await?.into_result()?;

    let metadata = FungibleTokenMetadata {
        spec: "ft-1.0.0".to_string(),
        name: vault_name.to_string(),
        symbol: vault_symbol.to_string(),
        icon: None,
        reference: None,
        reference_hash: None,
        decimals: 24,
    };

    contract
        .call("new")
        .args_json(json!({
            "asset": asset_contract.id(),
            "asset_token_id": asset_token_id,
            "metadata": metadata,
        }))
        .transact()
        .await?
        .into_result()?;

    // Note: MT contracts may not require storage registration like FT contracts

    Ok(contract)
}

pub async fn vault_storage_deposit(
    contract: &Contract,
    account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    account
        .call(contract.id(), "storage_deposit")
        .args_json(json!({
            "account_id": account.id(),
            "registration_only": false,
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

pub async fn mt_transfer_call_deposit(
    mt_contract: &Contract,
    vault_contract: &Contract,
    sender: &Account,
    token_id: &str,
    amount: u128,
    receiver_id: Option<&Account>,
    min_shares: Option<u128>,
    max_shares: Option<u128>,
    memo: Option<&str>,
) -> Result<U128, Box<dyn std::error::Error>> {
    let msg = if receiver_id.is_some()
        || min_shares.is_some()
        || max_shares.is_some()
        || memo.is_some()
    {
        json!({
            "receiver_id": receiver_id.map(|acc| acc.id()),
            "min_shares": min_shares.map(|s| s.to_string()),
            "max_shares": max_shares.map(|s| s.to_string()),
            "memo": memo,
        })
        .to_string()
    } else {
        "{}".to_string()
    };

    let result = sender
        .call(mt_contract.id(), "mt_transfer_call")
        .args_json(json!({
            "receiver_id": vault_contract.id(),
            "token_id": token_id,
            "amount": amount.to_string(),
            "msg": msg,
        }))
        .deposit(NearToken::from_yoctonear(0))
        .gas(near_workspaces::types::Gas::from_tgas(300))
        .transact()
        .await?
        .into_result()?;

    Ok(result.json()?)
}

pub async fn vault_redeem(
    vault_contract: &Contract,
    account: &Account,
    shares: u128,
    receiver_id: Option<&Account>,
    memo: Option<&str>,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result = account
        .call(vault_contract.id(), "redeem")
        .args_json(json!({
            "shares": shares.to_string(),
            "receiver_id": receiver_id.map(|acc| acc.id()),
            "memo": memo,
        }))
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_workspaces::types::Gas::from_tgas(300))
        .transact()
        .await?
        .into_result()?;

    Ok(result.json()?)
}

pub async fn vault_withdraw(
    vault_contract: &Contract,
    account: &Account,
    assets: u128,
    receiver_id: Option<&Account>,
    memo: Option<&str>,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result = account
        .call(vault_contract.id(), "withdraw")
        .args_json(json!({
            "assets": assets.to_string(),
            "receiver_id": receiver_id.map(|acc| acc.id()),
            "memo": memo,
        }))
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_workspaces::types::Gas::from_tgas(300))
        .transact()
        .await?
        .into_result()?;

    Ok(result.json()?)
}

pub async fn vault_total_assets(
    vault_contract: &Contract,
    account: &Account,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "total_assets")
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_convert_to_shares(
    vault_contract: &Contract,
    account: &Account,
    assets: u128,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "convert_to_shares")
        .args_json(json!({"assets": assets.to_string()}))
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_convert_to_assets(
    vault_contract: &Contract,
    account: &Account,
    shares: u128,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "convert_to_assets")
        .args_json(json!({"shares": shares.to_string()}))
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_preview_withdraw(
    vault_contract: &Contract,
    account: &Account,
    assets: u128,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "preview_withdraw")
        .args_json(json!({"assets": assets.to_string()}))
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_asset(
    vault_contract: &Contract,
    account: &Account,
) -> Result<String, Box<dyn std::error::Error>> {
    let result: String = account.view(vault_contract.id(), "asset").await?.json()?;
    Ok(result)
}

pub async fn vault_asset_token_id(
    vault_contract: &Contract,
    account: &Account,
) -> Result<String, Box<dyn std::error::Error>> {
    let result: String = account
        .view(vault_contract.id(), "asset_token_id")
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_balance_of(
    vault_contract: &Contract,
    account: &Account,
    account_id: &Account,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "ft_balance_of")
        .args_json(json!({"account_id": account_id.id()}))
        .await?
        .json()?;
    Ok(result)
}

pub async fn vault_total_supply(
    vault_contract: &Contract,
    account: &Account,
) -> Result<U128, Box<dyn std::error::Error>> {
    let result: U128 = account
        .view(vault_contract.id(), "ft_total_supply")
        .await?
        .json()?;
    Ok(result)
}
