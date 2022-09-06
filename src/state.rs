use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_utils::Expiration;
use cosmwasm_std::{Addr, Storage,BlockInfo};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static NAME_RESOLVER_KEY: &[u8] = b"nameresolver";
pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: String,
    pub wallet:Addr,
    pub cw721:Addr,
    pub base_cost:u64,
    pub base_expiration:Expiration
}

pub fn config(storage: &mut dyn Storage) -> Singleton<Config> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<Config> {
    singleton_read(storage, CONFIG_KEY)
}
/**
    add expiration
    and top level domain?
**/
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NameRecord {
    pub owner: Addr,
    pub expiration:Expiration
}
impl NameRecord {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expiration.is_expired(block)
    }
}
pub fn resolver(storage: &mut dyn Storage) -> Bucket<NameRecord> {
    bucket(storage, NAME_RESOLVER_KEY)
}

pub fn resolver_read(storage: &dyn Storage) -> ReadonlyBucket<NameRecord> {
    bucket_read(storage, NAME_RESOLVER_KEY)
}
