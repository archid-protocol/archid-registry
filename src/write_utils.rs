use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, StdResult,
    Uint128, WasmMsg,Env
};


use crate::read_utils::get_name_body;
use crate::read_utils::{ query_current_metadata};
use crate::state::{ resolver, NameRecord};
use archid_token::{
    ExecuteMsg as Cw721ExecuteMsg, Metadata, MintMsg, Subdomain, UpdateMetadataMsg,
};

pub static DENOM: &str = "aconst";

#[allow(clippy::too_many_arguments)]
pub fn add_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
    resolver: Addr,
    created: u64,
    expiry: u64,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, cw721, deps).unwrap();
    let mut subdomains: Vec<Subdomain> = current_metadata.subdomains.as_ref().unwrap().clone();
    subdomains.push(Subdomain {
        name: Some(subdomain),
        resolver: Some(resolver),
        minted: None,
        created: Some(created),
        expiry: Some(expiry),
    });
    current_metadata.subdomains = Some((*subdomains).to_vec());
    println!("{:?}", &current_metadata);
    let resp = send_data_update(&name, cw721, current_metadata)?;
    Ok(resp)
}
pub fn update_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    domain: &String,
    subdomain: &String,
    resolver: Addr,
    expiry: u64,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(domain, cw721, deps).unwrap();
    let mut subdomains: Vec<Subdomain> = current_metadata.subdomains.as_ref().unwrap().clone();
    let index = subdomains
        .iter()
        .position(|r| &r.clone().name.unwrap() == subdomain)
        .unwrap();
    subdomains[index].expiry = Some(expiry);
    subdomains[index].minted = None;
    subdomains[index].resolver = Some(resolver);
    current_metadata.subdomains = Some((*subdomains).to_vec());
    let resp = send_data_update(domain, cw721, current_metadata)?;
    Ok(resp)
}
pub fn update_subdomain_expiry(
    nft: Addr,
    deps: DepsMut,
    domain: String,
    subdomain: String,
    expiration: u64,
) -> StdResult<Vec<CosmosMsg>> {
    let mut messages = Vec::new();
    let domain_route: String = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    let domain_config: NameRecord = (resolver(deps.storage).may_load(key)?).unwrap();
    let record = NameRecord {
        resolver: domain_config.resolver.clone(),
        created: domain_config.created,
        expiration,
    };
    resolver(deps.storage).save(key, &record)?;
    let msg = update_subdomain_metadata(
        &deps,
        &nft,
        &domain,
        &subdomain,
        domain_config.resolver,
        expiration,
    )?;
    messages.push(msg);

    Ok(messages)
}
pub fn remove_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, cw721, deps).unwrap();
    let mut subdomains = current_metadata.subdomains.as_ref().unwrap().clone();

    subdomains.retain(|item| item.name.as_ref().unwrap().as_bytes() != subdomain.as_bytes());
    current_metadata.subdomains = Some((*subdomains).to_vec());
    let resp = send_data_update(&name, cw721, current_metadata)?;
    Ok(resp)
}

pub fn mint_handler(
    name: &String,
    creator: &Addr,
    cw721: &Addr,
    created: u64,
    expiration: u64,
) -> StdResult<CosmosMsg> {
    let body = get_name_body(name.to_string());
    let subdomains = if body.contains('.') {
        None
    } else {
        Some(vec![])
    };
    let accounts = if body.contains('.') {
        None
    } else {
        Some(vec![])
    };
    let websites = if body.contains('.') {
        None
    } else {
        Some(vec![])
    };
    let description = if body.contains('.') {
        [name, " subdomain"].concat()
    } else {
        [name, " domain"].concat()
    };

    let mint_extension = Some(Metadata {
        description: Some(description),
        name: Some(body),
        image: None,
        created: Some(created),
        expiry: Some(expiration),
        domain: Some(name.clone()),
        subdomains,
        accounts,
        websites,
    });

    let mint_msg: archid_token::ExecuteMsg = Cw721ExecuteMsg::Mint(MintMsg {
        token_id: name.to_string(),
        owner: creator.to_string(),
        token_uri: None,
        extension: mint_extension,
    });

    let resp: CosmosMsg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }
    .into();
    Ok(resp)
}

pub fn burn_handler(name: &String, cw721: &Addr) -> StdResult<CosmosMsg> {
    let burn_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::Burn {
        token_id: name.to_string(),
    };
    let resp: CosmosMsg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&burn_msg)?,
        funds: vec![],
    }
    .into();
    Ok(resp)
}

pub fn send_tokens(to: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: ([Coin {
            denom: String::from(DENOM),
            amount,
        }])
        .to_vec(),
    };
    Ok(msg.into())
}

pub fn send_data_update(name: &String, cw721: &Addr, data: Metadata) -> StdResult<CosmosMsg> {
    let update = Cw721ExecuteMsg::UpdateMetadata(UpdateMetadataMsg {
        token_id: name.to_string(),
        extension: Some(data),
    });
    let resp: CosmosMsg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&update)?,
        funds: vec![],
    }
    .into();
    Ok(resp)
}
pub fn update_metadata_expiry(
    deps: DepsMut,
    cw721: &Addr,
    name: String,
    expiration: u64,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, cw721, &deps).unwrap();
    current_metadata.expiry = Some(expiration);
    let resp = send_data_update(&name, cw721, current_metadata)?;
    Ok(resp)
}

#[allow(clippy::too_many_arguments)]
pub fn register_new_subdomain(
    nft: Addr,
    deps: DepsMut,
    env: Env,
    domain: String,
    subdomain: String,
    new_resolver: Addr,
    new_owner: Addr,
    expiration: u64,
) -> StdResult<Vec<CosmosMsg>> {
    let domain_route: String = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    let mut messages = Vec::new();
    let created = env.block.time.seconds();

    let metadata_msg = add_subdomain_metadata(
        &deps,
        &nft,
        domain,
        subdomain,
        new_resolver.clone(),
        env.block.time.seconds(),
        expiration,
    )?;
    messages.push(metadata_msg);
    let record = NameRecord {
        resolver: new_resolver,
        created,
        expiration,
    };
    resolver(deps.storage).save(key, &record)?;

    let resp = mint_handler(&domain_route, &new_owner, &nft, created, expiration)?;
    messages.push(resp);

    Ok(messages)
}
// can be used to mint and update resolver for non minted

#[allow(clippy::too_many_arguments)]
pub fn burn_remint_subdomain(
    deps: DepsMut,
    nft: Addr,
    env: Env,
    domain: String,
    subdomain: String,
    new_resolver: Addr,
    new_owner: Addr,
    expiration: u64,
) -> StdResult<Vec<CosmosMsg>> {
    let domain_route: String = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    let mut messages = Vec::new();
    let created = env.block.time.seconds();
    let burn_msg = burn_handler(&format!("{}.{}", subdomain, domain), &nft)?;
    messages.push(burn_msg);

    let metadata_msg = add_subdomain_metadata(
        &deps,
        &nft,
        domain,
        subdomain,
        new_resolver.clone(),
        env.block.time.seconds(),
        expiration,
    )?;
    messages.push(metadata_msg);
    let record = NameRecord {
        resolver: new_resolver,
        created,
        expiration,
    };
    resolver(deps.storage).save(key, &record)?;

    let resp = mint_handler(&domain_route, &new_owner, &nft, created, expiration)?;
    messages.push(resp);

    Ok(messages)
}