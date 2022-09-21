use crate::state::Config;
use cosmwasm_std::Addr;
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub wallet: Addr,
    pub cw721: Addr,
    pub base_cost: u64,
    pub base_expiration: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register {
        name: String,
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
    UpdateConfig {
        update_config: Config,
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
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RecordExpirationResponse {
    pub expiration: u64,
}
