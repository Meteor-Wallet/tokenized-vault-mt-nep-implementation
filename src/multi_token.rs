use near_sdk::{ext_contract, json_types::U128, AccountId, PromiseOrValue};

/// Core trait for NEP-245 Multi Token standard
/// This is a minimal implementation focused on the needs of the vault contract
pub trait MultiTokenCore {
    /// Transfer a specific amount of token_id from predecessor to receiver_id
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        approval: Option<u64>,
        memo: Option<String>,
    );

    /// Transfer tokens and call a method on the receiver contract
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        approval: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;

    /// Get balance of account_id for token_id
    fn mt_balance_of(&self, account_id: AccountId, token_id: String) -> U128;

    /// Get total supply of token_id
    fn mt_supply(&self, token_id: String) -> Option<U128>;
}

/// Receiver trait for NEP-245 Multi Token standard
/// Contracts implementing this can receive multi-token transfers
pub trait MultiTokenReceiver {
    /// Called when tokens are transferred to this contract via mt_transfer_call
    /// Returns the amount of tokens used (the rest is returned to sender)
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_ids: Vec<String>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}

/// External contract interface for making cross-contract calls to NEP-245 contracts
#[ext_contract(ext_mt_core)]
pub trait _ExtMultiTokenCore {
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        approval: Option<u64>,
        memo: Option<String>,
    );
}
