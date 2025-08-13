#[tokio::test]
async fn test_contract_compilation() -> Result<(), Box<dyn std::error::Error>> {
    near_workspaces::compile_project("./").await?;

    Ok(())
}

#[tokio::test]
async fn test_mock_mt_contract_compilation() -> Result<(), Box<dyn std::error::Error>> {
    near_workspaces::compile_project("./mock_contracts/mock_mt").await?;

    Ok(())
}
