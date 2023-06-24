use crate::state::{Config};
use archid_token::{Account, Website};
use cosmwasm_std::{Addr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
    ExtendSubdomainExpiry {
        domain: String,
        subdomain: String,
        expiration: u64,
    },
    UpdateResolver {
        name: String,
        new_resolver: Addr,
    },
    RegisterSubdomain {
        domain: String,
        subdomain: String,
        new_resolver: Addr,
        new_owner: Addr,
        expiration: u64,
    },
    RemoveSubdomain {
        domain: String,
        subdomain: String,
    },
    UpdateConfig {
        config: Config,
    },
    UpdateUserDomainData {
        name: String,
        metadata_update: MetaDataUpdateMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ResolveRecord { name: String },
    RecordExpiration { name: String },
    ResolveAddress { address: Addr },
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ResolveRecordResponse {
    pub address: Option<String>,
    pub expiration: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ResolveAddressResponse {
    // pub names: Option<Vec<Record<NameRecord>>>,
    pub names: Option<Vec<String>>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct RecordExpirationResponse {
    pub created: u64,
    pub expiration: u64,
}
