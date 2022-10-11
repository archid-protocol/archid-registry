use crate::state::Config;
use archid_token::{Account, Website};
use cosmwasm_std::{Addr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub wallet: Addr,
    pub cw721: Addr,
    pub base_cost: Uint128,
    pub base_expiration: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MetaDataUpdateMsg {
    pub description: Option<String>,
    pub image: Option<String>,
    pub accounts: Option<Vec<Account>>,
    pub websites: Option<Vec<Website>>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register {
        name: String,
    },
    Withdraw {
        amount: Uint128,
    },
    RenewRegistration {
        name: String,
    },
    UpdateResolver {
        name: String,
        new_resolver: Addr,
    },

    RegisterSubDomain {
        domain: String,
        subdomain: String,
        new_resolver: Addr,
        mint: bool,
        expiration: u64,
    },
    RemoveSubDomain {
        domain: String,
        subdomain: String,    
    },
    UpdateConfig {
        update_config: Config,
    },
    UpdataUserDomainData {
        name: String,
        metadata_update: MetaDataUpdateMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ResolveRecord { name: String },
    RecordExpiration { name: String },
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ResolveRecordResponse {
    pub address: Option<String>,
    pub expiration: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RecordExpirationResponse {
    pub expiration: u64,
}
