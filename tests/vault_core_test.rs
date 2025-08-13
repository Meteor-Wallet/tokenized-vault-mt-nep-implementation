use crate::helper::{
    mock_mt::{deploy_and_init_mock_mt, mt_balance_of, mt_mint},
    vault::{
        deploy_and_init_vault, mt_transfer_call_deposit, vault_asset, vault_asset_token_id,
        vault_balance_of, vault_convert_to_assets, vault_convert_to_shares, vault_preview_withdraw,
        vault_redeem, vault_storage_deposit, vault_total_assets, vault_total_supply,
        vault_withdraw,
    },
};

mod helper;

/// Test basic vault initialization and metadata
#[tokio::test]
async fn test_vault_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Test asset() returns correct underlying asset
    let asset_address = vault_asset(&vault, &owner).await?;
    assert_eq!(asset_address, usdt.id().to_string());

    // Test asset_token_id() returns correct token ID
    let token_id = vault_asset_token_id(&vault, &owner).await?;
    assert_eq!(token_id, "token1");

    // Test initial total_assets is 0
    let total_assets = vault_total_assets(&vault, &owner).await?;
    assert_eq!(total_assets.0, 0);

    // Test initial total supply is 0
    let total_supply = vault_total_supply(&vault, &owner).await?;
    assert_eq!(total_supply.0, 0);

    Ok(())
}

/// Test deposit functionality via ft_transfer_call
#[tokio::test]
async fn test_deposit_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Test deposit
    let deposit_amount = 1000u128;
    let result = mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        None,
        None,
    )
    .await?;

    // Verify result is 1000 (used amount) - ft_resolve_transfer returns used amount
    assert_eq!(result.0, 1000);

    // Verify vault state
    let total_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(total_assets.0, deposit_amount);

    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_shares.0, deposit_amount); // 1:1 ratio for first deposit

    let total_supply = vault_total_supply(&vault, &alice).await?;
    assert_eq!(total_supply.0, deposit_amount);

    Ok(())
}

/// Test conversion functions (convert_to_shares and convert_to_assets)
#[tokio::test]
async fn test_conversion_functions() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit
    let deposit_amount = 1000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        None,
        None,
    )
    .await?;

    // Test conversion functions with 1:1 ratio (adjusted for inflation resistance)
    let shares_for_500_assets = vault_convert_to_shares(&vault, &alice, 500).await?;
    assert_eq!(shares_for_500_assets.0, 499);

    let assets_for_500_shares = vault_convert_to_assets(&vault, &alice, 500).await?;
    assert_eq!(assets_for_500_shares.0, 500);

    Ok(())
}

/// Test redeem functionality (burn shares for assets)
/// This test verifies that users can redeem shares to get back assets.
#[tokio::test]
async fn test_redeem_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit
    let deposit_amount = 1000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        None,
        None,
    )
    .await?;

    let initial_alice_ft_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let initial_alice_shares = vault_balance_of(&vault, &alice, &alice).await?.0;

    // Redeem half the shares
    let redeem_shares = 500u128;
    let assets_received = vault_redeem(&vault, &alice, redeem_shares, None, None).await?;

    // In a properly working vault, redeem should return approximately the proportional asset amount
    // Expected: 500 shares should redeem for ~500 assets (1:1 ratio in this case)
    assert!(
        assets_received.0 >= 498 && assets_received.0 <= 502,
        "Redeem should return proportional assets (expected ~500, got {})",
        assets_received.0
    );

    // User should receive the redeemed assets
    let final_alice_ft_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let expected_balance = initial_alice_ft_balance + assets_received.0;
    assert_eq!(
        final_alice_ft_balance, expected_balance,
        "User should receive redeemed assets (expected {}, got {})",
        expected_balance, final_alice_ft_balance
    );

    // Shares should be burned during successful redeem
    let final_alice_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    let expected_shares = initial_alice_shares - redeem_shares;
    assert_eq!(
        final_alice_shares, expected_shares,
        "Shares should be burned during redeem (expected {}, got {})",
        expected_shares, final_alice_shares
    );

    // Vault state should reflect the redemption
    let final_total_assets = vault_total_assets(&vault, &alice).await?;
    let expected_total_assets = 1000 - assets_received.0;
    assert_eq!(
        final_total_assets.0, expected_total_assets,
        "Total assets should decrease by redeemed amount (expected {}, got {})",
        expected_total_assets, final_total_assets.0
    );

    let final_total_supply = vault_total_supply(&vault, &alice).await?;
    let expected_total_supply = 1000 - redeem_shares;
    assert_eq!(
        final_total_supply.0, expected_total_supply,
        "Total supply should decrease by burned shares (expected {}, got {})",
        expected_total_supply, final_total_supply.0
    );

    Ok(())
}

/// Test withdraw functionality (burn shares to get specific asset amount)
/// This test verifies that users can withdraw a specific amount of assets.
#[tokio::test]
async fn test_withdraw_functionality() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit
    let deposit_amount = 1000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        None,
        None,
    )
    .await?;

    let initial_alice_ft_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let initial_alice_shares = vault_balance_of(&vault, &alice, &alice).await?.0;

    // Withdraw specific asset amount
    let withdraw_assets = 500u128;
    let shares_used = vault_withdraw(&vault, &alice, withdraw_assets, None, None).await?;

    // In a properly working vault, withdraw should burn approximately the required shares to get the asset amount
    // Expected: ~500 shares should be burned to withdraw 500 assets (1:1 ratio in this case)
    assert!(
        shares_used.0 >= 498 && shares_used.0 <= 502,
        "Withdraw should burn proportional shares to get desired assets (expected ~500, got {})",
        shares_used.0
    );

    // User should receive exactly the requested asset amount
    let final_alice_ft_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let expected_balance = initial_alice_ft_balance + withdraw_assets;
    assert_eq!(
        final_alice_ft_balance, expected_balance,
        "User should receive exactly the requested withdrawal amount (expected {}, got {})",
        expected_balance, final_alice_ft_balance
    );

    // Shares should be burned during successful withdraw
    let final_alice_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    let expected_shares = initial_alice_shares - shares_used.0;
    assert_eq!(
        final_alice_shares, expected_shares,
        "Shares should be burned during withdraw (expected {}, got {})",
        expected_shares, final_alice_shares
    );

    // Vault state should reflect the withdrawal
    let final_total_assets = vault_total_assets(&vault, &alice).await?;
    let expected_total_assets = 1000 - withdraw_assets;
    assert_eq!(
        final_total_assets.0, expected_total_assets,
        "Total assets should decrease by withdrawn amount (expected {}, got {})",
        expected_total_assets, final_total_assets.0
    );

    let final_total_supply = vault_total_supply(&vault, &alice).await?;
    let expected_total_supply = 1000 - shares_used.0;
    assert_eq!(
        final_total_supply.0, expected_total_supply,
        "Total supply should decrease by burned shares (expected {}, got {})",
        expected_total_supply, final_total_supply.0
    );

    Ok(())
}

/// Test preview_withdraw function
/// This test verifies that preview_withdraw correctly calculates required shares.
#[tokio::test]
async fn test_preview_withdraw() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit
    let deposit_amount = 1000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        None,
        None,
    )
    .await?;

    // Test preview_withdraw
    let preview_shares = vault_preview_withdraw(&vault, &alice, 500).await?;
    // 500 * 1000 / 1001 = 499.5, rounded up = 500 shares
    assert_eq!(preview_shares.0, 500);

    // In a properly working vault, actual withdraw should match the preview calculation
    let actual_shares_used = vault_withdraw(&vault, &alice, 500, None, None).await?;
    assert_eq!(
        actual_shares_used.0, preview_shares.0,
        "Actual withdraw should use exactly the shares predicted by preview (expected {}, got {})",
        preview_shares.0, actual_shares_used.0
    );

    // Verify the preview calculation remains consistent
    assert_eq!(
        preview_shares.0, 500,
        "Preview should calculate correctly: 500 assets should require ~500 shares"
    );

    Ok(())
}

/// Test deposit with receiver_id parameter
#[tokio::test]
async fn test_deposit_with_receiver() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit like FT contracts
    vault_storage_deposit(&vault, &alice).await?;
    vault_storage_deposit(&vault, &bob).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Alice deposits but shares go to Bob
    let deposit_amount = 1000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        Some(&bob),
        None,
        None,
        None,
    )
    .await?;

    // Verify alice has no shares
    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_shares.0, 0);

    // Verify bob received the shares
    let bob_shares = vault_balance_of(&vault, &alice, &bob).await?;
    assert_eq!(bob_shares.0, deposit_amount);

    Ok(())
}

/// Test deposit with min_shares and max_shares parameters  
#[tokio::test]
async fn test_deposit_with_slippage_protection() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Test with min_shares that should pass
    let deposit_amount = 1000u128;
    let min_shares = 900u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        Some(min_shares),
        None,
        None,
    )
    .await?;

    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_shares.0, deposit_amount);

    Ok(())
}

/// Test deposit with max_shares that should refund excess
#[tokio::test]
async fn test_deposit_max_shares_with_refund() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Alice tries to deposit 1000 USDT but sets max_shares to 500 (should only mint 500 shares, refund the rest)
    let deposit_amount = 1000u128;
    let max_shares = 500u128;
    let result = mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,
        None,
        Some(max_shares),
        None,
    )
    .await?;

    // Only 500 shares minted, so only 500 USDT used, 500 refunded
    assert_eq!(result.0, 500);

    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_shares.0, 500);

    let alice_ft_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    // She started with 10000, deposited 1000, but 500 refunded, so should have 9500
    assert_eq!(alice_ft_balance, 9500);

    let total_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(total_assets.0, 500);

    let total_supply = vault_total_supply(&vault, &alice).await?;
    assert_eq!(total_supply.0, 500);

    Ok(())
}

/// Test multiple users with same conversion rates
#[tokio::test]
async fn test_multi_user_same_rates() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit like FT contracts
    vault_storage_deposit(&vault, &alice).await?;
    vault_storage_deposit(&vault, &bob).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;
    mt_mint(&usdt, &bob, "token1", 10000).await?;

    // Alice deposits first (1:1 ratio)
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;

    // Bob deposits same amount at same rate
    mt_transfer_call_deposit(&usdt, &vault, &bob, "token1", 1000, None, None, None, None).await?;

    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    let bob_shares = vault_balance_of(&vault, &alice, &bob).await?;

    assert_eq!(alice_shares.0, 1000);
    assert_eq!(bob_shares.0, 999); // Due to inflation resistance adjustment

    // Total assets should be 2000
    let total_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(total_assets.0, 2000);

    // Total supply should be 1999
    let total_supply = vault_total_supply(&vault, &alice).await?;
    assert_eq!(total_supply.0, 1999);

    Ok(())
}

/// Test that asset() function returns correct underlying asset and never reverts
#[tokio::test]
async fn test_asset_function_properties() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // asset() should never revert and always return the same value
    let asset1 = vault_asset(&vault, &owner).await?;
    let asset2 = vault_asset(&vault, &owner).await?;

    assert_eq!(asset1, asset2);
    assert_eq!(asset1, usdt.id().to_string());

    Ok(())
}

/// Test deposit with wrong token ID fails  
#[tokio::test]
async fn test_deposit_wrong_token_id_fails() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let mt_contract = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(
        &owner,
        &mt_contract,
        "game_asset_1",
        "Game Asset Vault",
        "vGA1",
    )
    .await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&mt_contract, &alice, "game_asset_1", 10000).await?;
    mt_mint(&mt_contract, &alice, "wrong_token_id", 10000).await?;

    // Verify vault is configured for specific token ID
    let configured_token_id = vault_asset_token_id(&vault, &alice).await?;
    assert_eq!(configured_token_id, "game_asset_1");

    // Test deposit with wrong token ID - should fail
    let result = mt_transfer_call_deposit(
        &mt_contract,
        &vault,
        &alice,
        "wrong_token_id",
        1000,
        None,
        None,
        None,
        None,
    )
    .await;

    // Deposit should return 0 (no tokens used) due to token ID mismatch
    let result = result.expect("Transaction should succeed but use 0 tokens");
    assert_eq!(
        result.0, 0,
        "Deposit with wrong token ID should use 0 tokens"
    );

    // Verify no shares were minted
    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(
        alice_shares.0, 0,
        "No shares should be minted for wrong token ID"
    );

    Ok(())
}

/// Test that total_assets() never reverts
#[tokio::test]
async fn test_total_assets_never_reverts() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // total_assets() should work at all times
    let initial_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(initial_assets.0, 0);

    // After deposit
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;
    let after_deposit = vault_total_assets(&vault, &alice).await?;
    assert_eq!(after_deposit.0, 1000);

    // After partial withdrawal - should now work correctly
    let redeem_result = vault_redeem(&vault, &alice, 250, None, None).await?;
    assert_eq!(
        redeem_result.0, 250,
        "Redeem should return the redeemed assets"
    );

    let after_withdraw = vault_total_assets(&vault, &alice).await?;
    assert_eq!(
        after_withdraw.0, 750,
        "Assets should be reduced by withdrawn amount"
    );

    Ok(())
}
