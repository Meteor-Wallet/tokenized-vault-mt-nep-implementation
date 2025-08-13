use std::collections::HashMap;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::LookupMap,
    env, near_bindgen,
    AccountId, PanicOnDefault, Gas, Promise, PromiseResult,
};
use near_sdk::{json_types::U128, BorshStorageKey};

// Type alias for consistency
type TokenId = String;
type Approval = Option<u64>;

const GAS_FOR_MT_ON_TRANSFER: Gas = Gas::from_tgas(50);
const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_tgas(50);

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TokenBalances,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MockMultiToken {
    /// Maps from (account_id, token_id) to balance
    balances: LookupMap<String, U128>,
    /// Token supplies by token_id
    supplies: HashMap<String, U128>,
}

#[near_bindgen]
impl MockMultiToken {
    #[init]
    pub fn new() -> Self {
        Self {
            balances: LookupMap::new(StorageKey::TokenBalances),
            supplies: HashMap::new(),
        }
    }

    /// Mint tokens to an account (for testing purposes)
    pub fn mint(&mut self, account_id: AccountId, token_id: String, amount: U128) {
        let key = format!("{}:{}", account_id, token_id);
        let current_balance = self.balances.get(&key).unwrap_or(U128(0));
        self.balances.insert(&key, &U128(current_balance.0 + amount.0));
        
        let current_supply = self.supplies.get(&token_id).unwrap_or(&U128(0));
        self.supplies.insert(token_id, U128(current_supply.0 + amount.0));
    }

    // Multi-token core methods
    #[payable]
    pub fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        _approval: Approval,
        _memo: Option<String>,
    ) {
        let sender = env::predecessor_account_id();
        let sender_key = format!("{}:{}", sender, token_id);
        let receiver_key = format!("{}:{}", receiver_id, token_id);

        // Validate token exists
        assert!(self.supplies.contains_key(&token_id), "Token does not exist");

        // Simulate storage deposit requirement - fail if receiver is "nonexistent.testnet"
        // This simulates realistic NEP-245 behavior where accounts need to exist
        assert!(
            receiver_id.as_str() != "nonexistent.testnet",
            "Account does not exist or has no storage deposit"
        );

        let sender_balance = self.balances.get(&sender_key).unwrap_or(U128(0));
        assert!(sender_balance.0 >= amount.0, "Insufficient balance");

        // Perform the transfer
        self.balances.insert(&sender_key, &U128(sender_balance.0 - amount.0));
        
        let receiver_balance = self.balances.get(&receiver_key).unwrap_or(U128(0));
        self.balances.insert(&receiver_key, &U128(receiver_balance.0 + amount.0));
    }

    pub fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Approval,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        let sender = env::predecessor_account_id();
        self.mt_transfer(receiver_id.clone(), token_id.clone(), amount, approval, memo.clone());

        // Call the receiver contract's mt_on_transfer method
        Promise::new(receiver_id.clone())
            .function_call(
                "mt_on_transfer".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "sender_id": sender,
                    "previous_owner_id": sender.clone(),
                    "token_ids": vec![token_id.clone()],
                    "amounts": vec![amount],
                    "msg": msg
                })).unwrap(),
                near_sdk::NearToken::from_yoctonear(0),
                GAS_FOR_MT_ON_TRANSFER,
            )
            .then(
                Promise::new(env::current_account_id())
                    .function_call(
                        "mt_resolve_transfer".to_string(),
                        near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                            "sender_id": sender,
                            "receiver_id": receiver_id,
                            "token_id": token_id,
                            "amount": amount
                        })).unwrap(),
                        near_sdk::NearToken::from_yoctonear(0),
                        GAS_FOR_RESOLVE_TRANSFER,
                    )
            )
    }

    #[private]
    pub fn mt_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
    ) -> U128 {
        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                // Try to parse the result as unused amounts Vec<U128>
                if let Ok(unused_amounts) = near_sdk::serde_json::from_slice::<Vec<U128>>(&result) {
                    if let Some(unused) = unused_amounts.first() {
                        if unused.0 > 0 {
                            // Refund unused tokens
                            let sender_key = format!("{}:{}", sender_id, token_id);
                            let receiver_key = format!("{}:{}", receiver_id, token_id);
                            
                            let receiver_balance = self.balances.get(&receiver_key).unwrap_or(U128(0));
                            let sender_balance = self.balances.get(&sender_key).unwrap_or(U128(0));
                            
                            self.balances.insert(&receiver_key, &U128(receiver_balance.0 - unused.0));
                            self.balances.insert(&sender_key, &U128(sender_balance.0 + unused.0));
                        }
                        // Return amount used (total - unused)
                        return U128(amount.0 - unused.0);
                    }
                }
                // No unused amounts vector returned, assume all was used
                amount
            }
            PromiseResult::Failed => {
                // Transfer failed, refund all tokens
                let sender_key = format!("{}:{}", sender_id, token_id);
                let receiver_key = format!("{}:{}", receiver_id, token_id);
                
                let receiver_balance = self.balances.get(&receiver_key).unwrap_or(U128(0));
                let sender_balance = self.balances.get(&sender_key).unwrap_or(U128(0));
                
                self.balances.insert(&receiver_key, &U128(receiver_balance.0 - amount.0));
                self.balances.insert(&sender_key, &U128(sender_balance.0 + amount.0));
                
                // Transfer failed, so nothing was used
                U128(0)
            }
        }
    }

    pub fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128 {
        let key = format!("{}:{}", account_id, token_id);
        self.balances.get(&key).unwrap_or(U128(0))
    }

    pub fn mt_supply(&self, token_id: TokenId) -> Option<U128> {
        self.supplies.get(&token_id).cloned()
    }

    pub fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<bool> {
        token_ids
            .into_iter()
            .map(|token_id| self.supplies.contains_key(&token_id))
            .collect()
    }
}