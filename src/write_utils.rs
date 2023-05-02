use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::MetaDataUpdateMsg;
use crate::read_utils::get_name_body;
use crate::read_utils::{query_current_metadata, query_name_owner,is_expired};
use crate::state::{config_read, mint_status, resolver, Config};
use archid_token::{
    ExecuteMsg as Cw721ExecuteMsg, Metadata, MintMsg, Subdomain, UpdateMetadataMsg,
};

pub static DENOM: &str = "uconst";
pub fn add_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
    resolver: Addr,
    created: u64,
    expiry: u64,
    minted: bool,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, cw721, deps).unwrap();
    let mut subdomains: Vec<Subdomain> = current_metadata.subdomains.as_ref().unwrap().clone();
    subdomains.push(Subdomain {
        name: Some(subdomain),
        resolver: Some(resolver),
        minted: Some(minted),
        created: Some(created),
        expiry: Some(expiry),
    });
    current_metadata.subdomains = Some((*subdomains).to_vec());
    let resp = send_data_update(&name, cw721, current_metadata)?;
    Ok(resp)
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

pub fn user_metadata_update_handler(
    info: MessageInfo,
    deps: DepsMut,
    name: String,
    update: MetaDataUpdateMsg,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    let cw721 = c.cw721;
    let owner_response = query_name_owner(&name, &cw721, &deps).unwrap();

    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    let new_metadata = Metadata {
        description: update.clone().description,
        name: Some(name.clone()),
        image: update.clone().image,
        created: current_metadata.created,
        expiry: current_metadata.expiry,
        domain: current_metadata.domain,
        subdomains: current_metadata.subdomains,
        accounts: update.accounts,
        websites: update.websites,
    };
    let resp = send_data_update(&name, &cw721, new_metadata);
    Ok(Response::new().add_message(resp.unwrap()))
}

pub fn remove_subdomain(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    domain: String,
    subdomain: String,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    let domain_route = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    let mut messages = Vec::new();
    let has_minted: bool = mint_status(deps.storage).may_load(key)?.is_some();
    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
    resolver(deps.storage).remove(key);
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if has_minted {
        let subdomain_owner = query_name_owner(&domain_route, &c.cw721, &deps).unwrap();
        // if owner of the minted subdomain is not owner of the top level domain
        // and subdomain is not expired
        if !is_expired(&deps,key,&env.block)
            && subdomain_owner.owner != info.sender
        {
            return Err(ContractError::NameTaken { name: domain_route });
        }
        messages.push(remove_subdomain_metadata(&deps, &c.cw721, domain, subdomain).unwrap());
        messages.push(burn_handler(&domain_route, &c.cw721)?);
        mint_status(deps.storage).remove(key);
    }
    
    Ok(Response::new().add_messages(messages))
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
