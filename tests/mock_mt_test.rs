use crate::helper::mock_mt::{deploy_and_init_mock_mt, mt_balance_of, mt_mint};

mod helper;

#[tokio::test]
async fn test_mock_mt_contract_is_working() -> Result<(), Box<dyn std::error::Error>> {
    let worker = near_workspaces::sandbox().await?;

    let trent = worker.dev_create_account().await?;
    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    let mt_contract = deploy_and_init_mock_mt(&trent).await?;

    let token_id = "nft_1";

    // Mint tokens to alice
    mt_mint(&mt_contract, &alice, token_id, 1000).await?;

    let alice_balance_before = mt_balance_of(&mt_contract, &alice, token_id).await?;
    assert_eq!(alice_balance_before, 1000);

    let bob_balance_before = mt_balance_of(&mt_contract, &bob, token_id).await?;
    assert_eq!(bob_balance_before, 0);

    // Mint more tokens to bob
    mt_mint(&mt_contract, &bob, token_id, 500).await?;

    let alice_balance_after = mt_balance_of(&mt_contract, &alice, token_id).await?;
    assert_eq!(alice_balance_after, 1000);

    let bob_balance_after = mt_balance_of(&mt_contract, &bob, token_id).await?;
    assert_eq!(bob_balance_after, 500);

    Ok(())
}
