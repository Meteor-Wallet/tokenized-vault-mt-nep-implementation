---
NEP: 0
Title: Vault NEP
Authors: JY Chew <edwardchew97@gmail.com>, Lee Hoe Mun <leehoemun@gmail.com>, Wade <wz.lim.00@gmail.com>, Steve Kok <kokchoquan@gmail.com>
Status: New
DiscussionsTo: https://github.com/nearprotocol/neps/pull/0000
Type: Contract Standard
Requires: 141
Version: 1.0.0
Created: 2025-08-08
LastUpdated: 2025-08-08
---

## Summary

This NEP proposes a standardized interface for implementing vault contracts on the NEAR Protocol, drawing inspiration from the ERC-4626 standard widely used on Ethereum. A vault contract allows users to deposit a specific token from a NEP-245 Multi Token (MT) contract into the vault, in exchange for which the vault issues shares that represent proportional ownership of the vault's assets.

The underlying asset is a single token ID from a NEP-245 compliant multi-token contract. This allows for vaults that manage semi-fungible tokens, gaming assets, or any other MT-based assets. When deposited, the vault mints new shares to the depositor based on the current exchange rate between the vault's total assets and total shares in circulation. Conversely, when a user redeems shares, the vault burns those shares and returns the equivalent amount of the underlying MT asset to the user.

The issued shares themselves are also NEP-141 compliant fungible tokens, enabling them to be freely transferred between accounts or traded on decentralized exchanges (DEXs). This compatibility allows vault shares to be integrated into broader DeFi ecosystems, enabling use cases such as collateral in lending protocols, liquidity provision, or composable yield strategies.

By standardizing the vault interface, this NEP aims to improve interoperability, reduce integration costs, and encourage consistent, secure practices for vault implementation across the NEAR ecosystem.

## Motivation

Vault contracts are a fundamental building block in modern DeFi, enabling users to pool assets for yield generation, liquidity provision, or other strategies while receiving tokenized shares that represent their proportional ownership. However, without a standardized interface, each vault implementation on NEAR may expose different method names, return formats, and accounting mechanisms, creating unnecessary friction for developers, integrators, and auditors.

A consistent vault standard, inspired by ERC-4626, would provide multiple benefits:

-   Interoperability – Wallets, DEXs, lending protocols, and other DeFi applications can integrate with any compliant vault without custom logic for each implementation.

-   Reduced Integration Costs – Developers and projects save time and resources by building once against the standard interface rather than creating one-off integrations.

-   Ecosystem Growth – Standardized vaults make it easier for new projects to leverage existing liquidity and composability, accelerating adoption across the NEAR DeFi ecosystem.

By introducing this NEP, we aim to align vault design on NEAR with proven best practices from other blockchain ecosystems while optimizing for the unique features and requirements of NEP-141 fungible tokens.

## Specification

### Contract Interface

The contract should implement the VaultCore trait.

```rust
/// Specification for a fungible token vault that issues NEP-141 compliant shares.
///
/// A MultiTokenVault accepts deposits of a specific token ID from an underlying 
/// NEP-245 compliant multi-token asset and issues NEP-141 compliant "shares" in return. 
/// These shares can be transferred and traded like any other NEP-141 token.
///
/// This trait extends:
/// - [`FungibleTokenCore`] to provide NEP-141 functionality for shares.
/// - [`MultiTokenReceiver`] to receive the underlying NEP-245 assets
pub trait VaultCore:
    FungibleTokenCore + MultiTokenReceiver
{
    // ----------------------------
    // Asset Information
    // ----------------------------

    /// Returns the [`AccountId`] of the underlying asset token contract.
    ///
    /// The asset **must** be NEP-245 compliant.
    /// Implementations should store this as an immutable configuration value.
    fn asset(&self) -> AccountId;

    /// Returns the token ID of the underlying multi-token asset.
    ///
    /// The vault manages a single token ID from the multi-token contract.
    /// Implementations should store this as an immutable configuration value.
    fn asset_token_id(&self) -> String;

    /// Returns the total amount of underlying assets represented by all shares in existence.
    ///
    /// **Important:**
    /// - Represents the vault's *total managed value*, not just assets held in the contract.
    /// - If assets are staked, lent, swapped, or deployed elsewhere, this should return
    ///   an **estimated total equivalent value**.
    /// - Must be denominated in the same units as [`Self::asset`].
    fn total_assets(&self) -> U128;

    // ----------------------------
    // Conversion Helpers
    // ----------------------------

    /// Converts an amount of underlying assets to the equivalent number of shares.
    ///
    /// This is a **purely view-only estimation** that:
    /// - Does not update state.
    /// - Ignores user-specific constraints such as deposit limits or fees.
    ///
    /// See also: [`Self::preview_deposit`] for a version that accounts for limits and fees.
    fn convert_to_shares(&self, assets: U128) -> U128;

    /// Converts an amount of shares to the equivalent amount of underlying assets.
    ///
    /// This is a **purely view-only estimation** that:
    /// - Does not update state.
    /// - Ignores withdrawal restrictions, fees, or penalties.
    ///
    /// See also: [`Self::preview_redeem`] for a version that accounts for real-world constraints.
    fn convert_to_assets(&self, shares: U128) -> U128;

    // ----------------------------
    // Deposit / Redemption Limits
    // ----------------------------

    /// Returns the maximum amount of underlying assets that `receiver_id` can deposit.
    ///
    /// This may depend on:
    /// - Vault capacity.
    /// - User-specific limits.
    /// - Current on-chain conditions.
    ///
    /// Implementations should return `U128::MAX` to signal "unlimited" deposits.
    fn max_deposit(&self, receiver_id: AccountId) -> U128;

    /// Simulates depositing exactly `assets` into the vault and returns the number of shares
    /// that would be minted to the receiver.
    ///
    /// Differs from [`Self::convert_to_shares`] by accounting for:
    /// - Per-user deposit limits.
    /// - Protocol-specific deposit fees.
    fn preview_deposit(&self, assets: U128) -> U128;

    /// Returns the maximum number of shares that `receiver_id` can mint.
    ///
    /// This may depend on:
    /// - Vault capacity.
    /// - User-specific limits.
    /// - Current on-chain conditions.
    ///
    /// Implementations should return `U128::MAX` to signal "unlimited" minting.
    fn max_mint(&self, receiver_id: AccountId) -> U128;

    /// Simulates minting exactly `shares` and returns the amount of underlying assets
    /// that would be required.
    ///
    /// Differs from [`Self::convert_to_assets`] by accounting for:
    /// - Per-user minting limits.
    /// - Protocol-specific minting fees.
    ///
    /// Useful for frontends to estimate the cost of minting shares.
    fn preview_mint(&self, shares: U128) -> U128;

    /// Returns the maximum number of shares that `owner_id` can redeem.
    ///
    /// This may depend on:
    /// - The owner's current share balance.
    /// - Vault withdrawal restrictions.
    /// - Lock-up periods or cooldowns.
    ///
    /// Implementations should return `0` if redemptions are currently disabled for the owner.
    fn max_redeem(&self, owner_id: AccountId) -> U128;

    /// Returns the maximum amount of assets that `owner_id` can withdraw.
    ///
    /// This may depend on:
    /// - The owner's share balance.
    /// - Current vault liquidity.
    /// - Withdrawal limits or cooldowns.
    fn max_withdraw(&self, owner_id: AccountId) -> U128;

    // ----------------------------
    // Redemption Operations
    // ----------------------------

    /// Redeems `shares` from the caller in exchange for the equivalent amount of underlying assets.
    ///
    /// - If `receiver_id` is `None`, defaults to sending assets to the caller.
    /// - Burns the caller's shares.
    /// - Returns the exact amount of assets redeemed.
    ///
    /// # Panics / Fails
    /// - If the caller's share balance is insufficient.
    /// - If withdrawal limits prevent the redemption.
    ///
    /// See also: [`Self::preview_redeem`].
    fn redeem(&mut self, shares: U128, receiver_id: Option<AccountId>) -> PromiseOrValue<U128>;

    /// Simulates redeeming `shares` into assets without executing the redemption.
    ///
    /// Differs from [`Self::convert_to_assets`] by factoring in:
    /// - The caller's current share balance.
    /// - Vault withdrawal limits.
    /// - Applicable fees or penalties.
    ///
    /// Useful for frontends to estimate redemption outcomes.
    fn preview_redeem(&self, shares: U128) -> U128;

    /// Withdraws exactly `assets` worth of underlying tokens from the vault.
    ///
    /// - If `receiver_id` is `None`, defaults to sending assets to the caller.
    /// - Burns the required number of shares to fulfill the withdrawal.
    ///
    /// # Panics / Fails
    /// - If the caller's share balance cannot cover the withdrawal.
    /// - If withdrawal limits or fees prevent the withdrawal.
    ///
    /// See also: [`Self::preview_withdraw`].
    fn withdraw(&mut self, assets: U128, receiver_id: Option<AccountId>) -> PromiseOrValue<U128>;

    /// Simulates withdrawing exactly `assets` worth of tokens without executing.
    ///
    /// Differs from [`Self::convert_to_shares`] by factoring in:
    /// - The caller's current share balance.
    /// - Vault withdrawal limits.
    /// - Applicable fees or penalties.
    ///
    /// Useful for frontends to preview required shares for a given withdrawal.
    fn preview_withdraw(&self, assets: U128) -> U128;
}
```

### Events

```rust
/// Event emitted when a deposit is received by the vault.
///
/// This follows the proposed NEP vault standard, referencing the ERC-4626 pattern.
/// Upon receiving assets, the vault mints and issues shares to the `owner_id`.
pub struct VaultDeposit {
    /// The account that sends the deposit (payer of the assets).
    pub sender_id: AccountId,

    /// The account that receives the minted shares.
    pub owner_id: AccountId,

    /// The token ID of the deposited multi-token asset.
    pub token_id: String,

    /// Amount of underlying assets deposited into the vault.
    pub assets: U128,

    /// Amount of shares minted and issued to `owner_id`.
    pub shares: U128,

    /// Optional memo provided by the sender for off-chain use.
    pub memo: Option<String>,
}

/// Event emitted when shares are redeemed from the vault.
///
/// Upon redemption, the vault burns the shares from `owner_id`
/// and transfers the equivalent assets to `receiver_id`.
pub struct VaultWithdraw {
    /// The account that owns the shares being redeemed (burned).
    pub owner_id: AccountId,

    /// The account receiving the underlying assets.
    pub receiver_id: AccountId,

    /// The token ID of the withdrawn multi-token asset.
    pub token_id: String,

    /// Amount of shares redeemed (burned from the vault).
    pub shares: U128,

    /// Amount of underlying assets withdrawn from the vault.
    pub assets: U128,

    /// Optional memo provided by the redeemer for off-chain use.
    pub memo: Option<String>,
}

```

## Reference Implementation

We have created an example implementation. [ [Github Link](https://github.com/Meteor-Wallet/near-erc4626-vault) ]

Besides that, we have also written a Rust Trait and Events that we wish to be merge with the near-contract-standards repo. [ [Contract Standards](https://github.com/Meteor-Wallet/near-erc4626-vault/tree/main/src/contract_standards) ]

We are still working hard on writting test case to test our example implementation

## Security Implications

### Exchange Rate Manipulation
Vaults allow dynamic exchange rates between shares and assets, calculated by dividing total vault assets by total issued shares. If the vault has a permissionless donation mechanism, it creates vulnerability to inflation attacks where attackers manipulate the rate by donating assets to inflate share values, potentially stealing funds from subsequent depositors. Vault deployers can protect against this attack by making an initial deposit of a non-trivial amount of the asset, such that price manipulation becomes infeasible.

### Cross-contract Calls
Redeem and withdraw functions perform cross-contract calls to transfer fungible tokens, creating opportunities for reentrancy attacks and state manipulation during asynchronous execution. Vaults should implement reentrancy protection through proper state management, proper callback security, and rollback mechanisms for failed operations.

### Rounding Direction Security
Vault calculations must consistently round in favor of the vault to prevent exploitation. When issuing shares for deposits or transferring assets for redemptions, round down; when calculating required shares or assets for specific amounts, round up. This asymmetric rounding prevents users from extracting value through repeated micro-transactions that exploit rounding errors and protects existing shareholders from value dilution.

### Oracle and External Price Dependencies
Vaults that rely on external price oracles or cross-contract calls for exchange rate updates face additional security risks in Near's asynchronous environment. Oracle updates create temporal windows where vaults operate with stale pricing data, potentially allowing exploitation. Implementations should include staleness checks, prevent operations during oracle updates, implement proper callback security, and consider fallback pricing mechanisms for oracle failures.

## Alternatives

[Explain any alternative designs that were considered and the rationale for not choosing them. Why your design is superior?]

## Future possibilities

### Multi-Token ID Support
Future vault implementations could extend this standard to support multiple token IDs from the same NEP-245 Multi Token contract as underlying assets within a single vault.

### Multi-Asset Vault Extensions
Future extensions could allow vaults to accept multiple assets for deposit and withdrawal. This would enable the standardization of LP vaults.

### Asynchronous Vault Operations
Future vault standards could introduce asynchronous deposit and withdrawal patterns through `request_deposit` and `request_withdraw` functions. This would enable integration with cross-chain protocols and real-world asset protocols.

## Consequences

[This section describes the consequences, after applying the decision. All consequences should be summarized here, not just the "positive" ones. Record any concerns raised throughout the NEP discussion.]

### Positive

-   p1

### Neutral

-   n1

### Negative

-   n1

### Backwards Compatibility

[All NEPs that introduce backwards incompatibilities must include a section describing these incompatibilities and their severity. Author must explain a proposes to deal with these incompatibilities. Submissions without a sufficient backwards compatibility treatise may be rejected outright.]

## Unresolved Issues (Optional)

[Explain any issues that warrant further discussion. Considerations

-   What parts of the design do you expect to resolve through the NEP process before this gets merged?
-   What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
-   What related issues do you consider out of scope for this NEP that could be addressed in the future independently of the solution that comes out of this NEP?]

## Changelog

[The changelog section provides historical context for how the NEP developed over time. Initial NEP submission should start with version 1.0.0, and all subsequent NEP extensions must follow [Semantic Versioning](https://semver.org/). Every version should have the benefits and concerns raised during the review. The author does not need to fill out this section for the initial draft. Instead, the assigned reviewers (Subject Matter Experts) should create the first version during the first technical review. After the final public call, the author should then finalize the last version of the decision context.]

### 1.0.0 - Initial Version

> Placeholder for the context about when and who approved this NEP version.

#### Benefits

> List of benefits filled by the Subject Matter Experts while reviewing this version:

-   Benefit 1
-   Benefit 2

#### Concerns

> Template for Subject Matter Experts review for this version:
> Status: New | Ongoing | Resolved

|   # | Concern | Resolution | Status |
| --: | :------ | :--------- | -----: |
|   1 |         |            |        |
|   2 |         |            |        |

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).