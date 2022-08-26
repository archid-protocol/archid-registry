use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr};
use cw_utils::Expiration;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub cw721: Addr,
    pub base_cost:u64,
    pub base_expiration:Expiration
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register { name: String },
    UpdateResolver {name: String, new_resolver:Addr},
    RegisterSubDomain {domain: String,subdomain: String, new_resolver:Addr,mint:bool,expiration:Expiration}
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ResolveRecord { name: String },
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ResolveRecordResponse {
    pub address: Option<String>,
}
