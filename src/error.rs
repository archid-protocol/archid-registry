use cosmwasm_std::{StdError, Uint128};
use cw_utils::PaymentError;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("InvalidInput")]
    InvalidInput {},

    #[error("InvalidPayment")]
    InvalidPayment { amount: Uint128 },
    #[error("Name does not exist (name {name})")]
    NameNotExists { name: String },

    #[error("Name has been taken (name {name})")]
    NameTaken { name: String },

    #[error("Name too short (length {length} min_length {min_length})")]
    NameTooShort { length: u64, min_length: u64 },

    #[error("Name too long (length {length} min_length {max_length})")]
    NameTooLong { length: u64, max_length: u64 },
    #[error("Name ownership is  is expired")]
    NameOwnershipExpired { name: String },
    #[error("Invalid character(char {c}")]
    InvalidCharacter { c: char },
}
