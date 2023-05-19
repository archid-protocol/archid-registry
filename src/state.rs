use cosmwasm_std::{Addr, BlockInfo, Storage, Timestamp, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static NAME_RESOLVER_KEY: &[u8] = b"nameresolver";
pub static SUBDOMAIN_MINTED: &[u8] = b"subdomain_minted";
pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub wallet: Addr,
    pub cw721: Addr,
    pub base_cost: Uint128,
    pub base_expiration: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum SubDomainStatus {
    //if subdomain in acive mint domain owner can only extend expiration up to domain expiration
    ExistingMintActive,
    // if subdomain expired owner can remint which will first burn existing nft
    ExistingMintExpired,    
    // if new subdomain owner can register and mint / not mint
    NewSubdomain
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct NameRecord {
    pub resolver: Addr,
    pub created: u64,
    pub expiration: u64,
}
impl NameRecord {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        Expiration::AtTime(Timestamp::from_seconds(self.expiration)).is_expired(block)
    }
}
pub fn resolver(storage: &mut dyn Storage) -> Bucket<NameRecord> {
    bucket(storage, NAME_RESOLVER_KEY)
}

pub fn resolver_read(storage: &dyn Storage) -> ReadonlyBucket<NameRecord> {
    bucket_read(storage, NAME_RESOLVER_KEY)
}
