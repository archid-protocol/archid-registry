use cosmwasm_std::{
    entry_point, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdError, StdResult, Timestamp, Uint128, WasmMsg,
    WasmQuery,
};
use cw721_updatable::{NftInfoResponse, OwnerOfResponse};

use archid_token::{
    ExecuteMsg as Cw721ExecuteMsg, Extension, Metadata, MintMsg, QueryMsg as Cw721QueryMsg,
    UpdateMetadataMsg,Subdomain
};
use cw_utils::{must_pay, Expiration};
use std::convert::TryFrom;
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MetaDataUpdateMsg, QueryMsg, RecordExpirationResponse,
    ResolveRecordResponse,
};
use crate::state::{config, config_read, mint_status, resolver, resolver_read, Config, NameRecord};
use crate::read_utils::{query_name_owner,query_resolver,query_resolver_expiration,validate_name,query_current_metadata};

pub fn add_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
    resolver:Addr,
    expiry:u64,
    minted:bool
    
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    let mut subdomains:Vec<Subdomain> =current_metadata.subdomains.as_ref().unwrap().clone();
    subdomains.push(Subdomain{name:Some(subdomain),resolver:Some(resolver),minted:Some(minted),expiry:Some(expiry)});
    current_metadata.subdomains=Some((*subdomains).to_vec());
    let resp = send_data_update(&name, &cw721, current_metadata)?;
    Ok(resp)
}
pub fn remove_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    let mut subdomains =current_metadata.subdomains.as_ref().unwrap().clone();
    
    subdomains.retain(|item| item.name.as_ref().unwrap().as_bytes() !=subdomain.as_bytes());
    current_metadata.subdomains=Some((*subdomains).to_vec());
    let resp = send_data_update(&name, &cw721, current_metadata)?;
    Ok(resp)
}


pub fn mint_handler(
    name: &String,
    creator: &Addr,
    cw721: &Addr,
    expiration: u64,
) -> StdResult<CosmosMsg> {
    let mint_extension = Some(Metadata {
        description: Some(String::from("An arch id domain")),
        name: Some(name.clone()),
        image: None,
        expiry: Some(Expiration::AtTime(Timestamp::from_seconds(expiration))),
        domain: Some(name.clone()),
        subdomains: Some(vec![]),
        accounts: Some(vec![]),
        websites: Some(vec![]),
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
    let burn_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::BurnAdminOnly {
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
        image: update.clone().description,
        expiry: current_metadata.expiry,
        domain: current_metadata.domain,
        subdomains: current_metadata.subdomains,
        accounts: update.clone().accounts,
        websites: update.clone().websites,
    };
    let resp = send_data_update(&name, &cw721, new_metadata);
    Ok(Response::new().add_message(resp.unwrap()))
}

pub fn remove_subdomain(info: MessageInfo,
    deps: DepsMut,
    env: Env,    
    domain: String,
    subdomain: String) -> Result<Response, ContractError> {
        let c: Config = config_read(deps.storage).load()?;
        let domain_route = format!("{}.{}", subdomain, domain);
        let key = domain_route.as_bytes();
        let mut messages = Vec::new();
        let has_minted: bool = mint_status(deps.storage).may_load(key)?.is_some();
        let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
       
        if owner_response.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        if has_minted {        
            if !((resolver(deps.storage).may_load(key)?)
                .unwrap()
                .is_expired(&env.block))
            {
                return Err(ContractError::NameTaken { name: domain_route });
            }
            messages.push(remove_subdomain_metadata(&deps,&c.cw721,domain.clone(),subdomain.clone()).unwrap());
            messages.push(burn_handler(&domain_route, &c.cw721)?);
        }
        resolver(deps.storage).remove(key);
        Ok(Response::new().add_messages(messages))
    }
pub fn send_tokens(to: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
        let msg = BankMsg::Send {
            to_address: to.into(),
            amount: (&[Coin {
                // denom: String::from("ARCH"),
                denom: String::from("CONST"),
                amount: amount,
            }])
                .to_vec(),
        };
        Ok(msg.into())
    }

  pub  fn send_data_update(name: &String, cw721: &Addr, data: Metadata) -> StdResult<CosmosMsg> {
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