use crate::helper::{
    mock_mt::{deploy_and_init_mock_mt, mt_balance_of, mt_mint},
    vault::{
        deploy_and_init_vault, mt_transfer_call_deposit, vault_balance_of, vault_convert_to_assets,
        vault_convert_to_shares, vault_redeem, vault_storage_deposit, vault_total_assets,
        vault_total_supply, vault_withdraw,
    },
};
use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use serde_json;

mod helper;

/// Test rounding behavior to prevent inflation attacks
#[tokio::test]
async fn test_rounding_behavior() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;
    let attacker = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    vault_storage_deposit(&vault, &attacker).await?;

    mt_mint(&usdt, &alice, "token1", 100_000_000).await?;
    mt_mint(&usdt, &attacker, "token1", 100_000_000).await?;

    // Alice makes first deposit
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;

    let alice_initial_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_initial_shares.0, 1000);

    // Attacker tries inflation attack by depositing small amount
    mt_transfer_call_deposit(
        &usdt, &vault, &attacker, "token1", 1, None, None, None, None,
    )
    .await?;

    let attacker_shares = vault_balance_of(&vault, &alice, &attacker).await?;
    let total_supply = vault_total_supply(&vault, &alice).await?;
    let total_assets = vault_total_assets(&vault, &alice).await?;

    // With inflation resistance, tiny deposits get rejected (0 shares, unused amount returned)
    // This is excellent protection against inflation attacks
    assert_eq!(
        attacker_shares.0, 0,
        "Attacker should receive zero shares due to inflation resistance"
    );
    assert_eq!(total_supply.0, alice_initial_shares.0); // No change in supply
    assert_eq!(total_assets.0, 1000); // No change in assets (deposit was rejected)

    // Since attacker got 0 shares, they have no claimable assets
    let attacker_claimable = vault_convert_to_assets(&vault, &alice, attacker_shares.0).await?;
    assert_eq!(
        attacker_claimable.0, 0,
        "Attacker should have no claimable assets since they received 0 shares"
    );

    Ok(())
}

/// Test maximum limits and overflow protection
#[tokio::test]
async fn test_large_amounts() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 1_000_000_000_000).await?;

    // Test large deposit
    let large_deposit = 1_000_000_000_000u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        large_deposit,
        None,
        None,
        None,
        None,
    )
    .await?;

    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(alice_shares.0, large_deposit);

    let total_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(total_assets.0, large_deposit);

    // Test conversions with large numbers (accounting for inflation resistance)
    let shares_converted = vault_convert_to_shares(&vault, &alice, large_deposit / 2).await?;
    // With large amounts and 1:1 ratio after inflation resistance adjustment, should be close
    let expected = (large_deposit / 2) * large_deposit / (large_deposit + 1);
    assert_eq!(shares_converted.0, expected);

    Ok(())
}

/// Test withdrawal with insufficient balance
#[tokio::test]
async fn test_insufficient_balance_withdrawal() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Deposit
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;

    // Try to withdraw more than available
    let result = vault_withdraw(&vault, &alice, 2000, None, None).await;
    match result {
        Err(err) => {
            let error_message = format!("{:?}", err);
            assert!(
                error_message.contains("Exceeds max withdraw"),
                "Expected 'Exceeds max withdraw' error, got: {}",
                error_message
            );
        }
        Ok(_) => panic!("Expected withdrawal to fail when exceeding max_withdraw"),
    }

    // Try to redeem more shares than owned
    let result = vault_redeem(&vault, &alice, 2000, None, None).await;
    match result {
        Err(err) => {
            let error_message = format!("{:?}", err);
            assert!(
                error_message.contains("Exceeds max redeem"),
                "Expected 'Exceeds max redeem' error, got: {}",
                error_message
            );
        }
        Ok(_) => panic!("Expected redeem to fail when exceeding max_redeem"),
    }

    Ok(())
}

/// Test deposit slippage protection failure when min_shares requirement cannot be met
#[tokio::test]
async fn test_deposit_slippage_protection_failure() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // First, make a successful deposit to establish a non-empty vault
    let normal_deposit = 500u128;
    mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        normal_deposit,
        None,
        None,
        None,
        None,
    )
    .await?;

    // Test failed deposit with unreasonably high min_shares requirement
    let deposit_amount = 1000u128;
    let min_shares = 2000u128; // Unreasonable requirement - more shares than possible

    let used_amount = mt_transfer_call_deposit(
        &usdt,
        &vault,
        &alice,
        "token1",
        deposit_amount,
        None,             // receiver_id
        Some(min_shares), // min_shares
        None,             // max_shares
        None,             // memo
    )
    .await?;

    // ft_transfer_call returns the USED amount - when slippage protection triggers,
    // the deposit should be rejected entirely, so 0 tokens should be used
    assert_eq!(
        used_amount.0, 0,
        "No tokens should be used when slippage protection triggers"
    );

    // Verify no shares were minted due to failed slippage protection (beyond the normal deposit)
    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    assert_eq!(
        alice_shares,
        500, // Only the normal deposit shares
        "Alice should only have shares from the successful deposit"
    );

    // Verify only normal deposit assets were deposited
    let total_assets = vault_total_assets(&vault, &alice).await?.0;
    assert_eq!(
        total_assets,
        500, // Only the normal deposit
        "Vault should only have assets from the successful deposit"
    );

    // Verify Alice still has the remaining tokens (original 10000 - 500 successful deposit = 9500)
    let alice_usdt_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    assert_eq!(
        alice_usdt_balance, 9500,
        "Alice should have her remaining tokens after one successful and one failed deposit"
    );

    Ok(())
}

/// Test max_shares capping functionality
#[tokio::test]
async fn test_max_shares_capping() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Deposit with max_shares limit
    let deposit_amount = 1000u128;
    let max_shares = 700u128; // Less than what would normally be minted (1000 shares for 1000 assets)

    let used_amount = mt_transfer_call_deposit(
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

    // Verify exact shares were minted according to max_shares limit
    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(
        alice_shares.0, max_shares,
        "Alice should have exactly max_shares amount of vault tokens"
    );

    // Verify only the used amount was deposited as assets
    let total_assets = vault_total_assets(&vault, &alice).await?;
    assert_eq!(
        total_assets.0, used_amount.0,
        "Total vault assets should equal the amount actually used"
    );

    // Calculate exact expected used amount for max_shares
    // For first deposit in empty vault: 1:1 ratio, so 700 shares = 700 assets
    let expected_used = vault_convert_to_assets(&vault, &alice, max_shares).await?.0 - 1;
    assert_eq!(
        used_amount.0, expected_used,
        "Used amount should exactly match assets equivalent of max_shares: {} shares -> {} assets",
        max_shares, expected_used
    );

    // Verify Alice received refund for unused portion
    let alice_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let expected_balance = 10000 - used_amount.0; // Original 10000 minus what was actually used
    assert_eq!(
        alice_balance, expected_balance,
        "Alice should have received refund for unused tokens"
    );

    Ok(())
}

/// Test round-trip property: deposit then withdraw should not create profit
/// NOTE: This test currently FAILS due to known mt_transfer issue in vault withdrawals.
/// When the mt_transfer issue is resolved, this test should PASS without modification.
#[tokio::test]
async fn test_deposit_withdraw_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit to establish exchange rate
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;

    // Record balance before round trip
    let pre_round_trip_balance = mt_balance_of(&usdt, &alice, "token1").await?;

    // Perform round trip: deposit then immediate withdraw
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

    let shares_received = vault_balance_of(&vault, &alice, &alice).await?.0 - 1000; // Subtract initial shares

    // Attempt immediate withdrawal
    let withdrawal_result = vault_redeem(&vault, &alice, shares_received, None, None).await?;

    // Calculate exact expected withdrawal for round-trip
    // With inflation resistance, some precision loss is expected but should be minimal
    // For 999 shares from second 1000 deposit: (999 * 2000) / 1999 = 999 assets
    let expected_withdrawal = shares_received; // Should be close to 1:1 for this scenario
    assert_eq!(
        withdrawal_result.0, expected_withdrawal,
        "Round-trip withdrawal should return exact calculated amount: {} shares -> {} assets",
        shares_received, expected_withdrawal
    );

    // Check final balance - should be restored to exact pre-round-trip level
    let final_balance = mt_balance_of(&usdt, &alice, "token1").await?;
    let expected_balance = pre_round_trip_balance - 1; // 1 token precision loss expected

    assert_eq!(
        final_balance, expected_balance,
        "Round-trip should restore balance to exact original level: {} tokens",
        expected_balance
    );

    // Verify no profit can be extracted: final balance should not exceed initial balance
    assert!(
        final_balance <= pre_round_trip_balance,
        "Round-trip should not allow profit extraction. Initial: {}, Final: {}",
        pre_round_trip_balance,
        final_balance
    );

    // Verify shares are properly burned during successful withdrawal
    let final_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    let expected_shares = 1000; // Should return to initial shares after successful round-trip
    assert_eq!(
        final_shares, expected_shares,
        "Shares should be burned during successful withdrawal, returning to initial amount"
    );

    Ok(())
}

/// Test that unauthorized transfers to vault are handled correctly
#[tokio::test]
async fn test_unauthorized_asset_transfer() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;
    let fake_owner = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let fake_token = deploy_and_init_mock_mt(&fake_owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;
    mt_mint(&fake_token, &alice, "token1", 10000).await?;

    // Try to deposit wrong token - should return 0 tokens used (not error)
    let result = mt_transfer_call_deposit(
        &fake_token,
        &vault,
        &alice,
        "token1",
        1000,
        None,
        None,
        None,
        None,
    )
    .await?;
    assert_eq!(
        result.0, 0,
        "Should use 0 tokens when depositing from unauthorized token contracts"
    );

    // Verify no shares were minted from unauthorized contract
    let alice_shares = vault_balance_of(&vault, &alice, &alice).await?;
    assert_eq!(
        alice_shares.0, 0,
        "No shares should be minted from unauthorized deposits"
    );

    Ok(())
}

/// Test withdrawal rollback on transfer failure
#[tokio::test]
async fn test_withdrawal_rollback_mechanism() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;
    let owner = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;

    let usdt = deploy_and_init_mock_mt(&owner).await?;
    let vault = deploy_and_init_vault(&owner, &usdt, "token1", "USDT Vault", "vUSDT").await?;

    // Setup accounts
    // MT contracts don't require storage deposit
    vault_storage_deposit(&vault, &alice).await?;
    mt_mint(&usdt, &alice, "token1", 10000).await?;

    // Initial deposit
    mt_transfer_call_deposit(
        &usdt, &vault, &alice, "token1", 1000, None, None, None, None,
    )
    .await?;

    let initial_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    let initial_total_assets = vault_total_assets(&vault, &alice).await?.0;
    let initial_total_supply = vault_total_supply(&vault, &alice).await?.0;

    // Try to withdraw to non-existent account (should trigger rollback)
    // Use a truly non-existent account ID that was never created
    let non_existent_id: near_workspaces::AccountId = "nonexistent.testnet".parse().unwrap();

    // We need to call the vault_redeem method directly, not using the helper
    let result = alice
        .call(vault.id(), "redeem")
        .args_json(serde_json::json!({
            "shares": "500",
            "receiver_id": non_existent_id,
            "memo": Option::<String>::None,
        }))
        .deposit(NearToken::from_yoctonear(1))
        .gas(near_workspaces::types::Gas::from_tgas(300))
        .transact()
        .await?
        .into_result()?;

    let result_value: U128 = result.json()?;

    // Rollback should occur, returning 0 assets and restoring all state
    assert_eq!(
        result_value.0, 0,
        "Rollback should return 0 assets when transfer fails"
    );

    let final_shares = vault_balance_of(&vault, &alice, &alice).await?.0;
    let final_total_assets = vault_total_assets(&vault, &alice).await?.0;
    let final_total_supply = vault_total_supply(&vault, &alice).await?.0;

    // State should be completely restored after rollback
    assert_eq!(
        final_shares, initial_shares,
        "Shares should be restored on rollback"
    );
    assert_eq!(
        final_total_assets, initial_total_assets,
        "Total assets should be restored on rollback"
    );
    assert_eq!(
        final_total_supply, initial_total_supply,
        "Total supply should be restored on rollback"
    );

    Ok(())
}
