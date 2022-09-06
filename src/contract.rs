use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut,QueryRequest, Env,Addr, MessageInfo, Response, StdError, StdResult,CosmosMsg,WasmMsg,WasmQuery
};
use std::ops::Add;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, Extension,
    MintMsg,msg::QueryMsg as Cw721QueryMsg
};
use cw721::{
    OwnerOfResponse
};
use cw_utils::{Expiration,Duration};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse};
use crate::state::{config, config_read, resolver, resolver_read, Config, NameRecord};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let config_state = Config {
        admin: deps.api.addr_validate(&msg.admin)?.into_string(),
        wallet:msg.wallet,
        cw721:msg.cw721,
        base_cost:msg.base_cost,
        base_expiration:msg.base_expiration
    };
    config(deps.storage).save(&config_state)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register { name } => execute_register(deps, env, info, name),
        ExecuteMsg::UpdateResolver {name,new_resolver} =>update_resolver(info,deps, env, name,new_resolver),
        ExecuteMsg::RegisterSubDomain {domain,subdomain, new_resolver,mint,expiration}=> set_subdomain( info,deps,env, domain,subdomain, new_resolver,mint,expiration),
    }
}

pub fn execute_register(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    validate_name(&name)?;
    let key = &name.as_bytes();
    let curr=resolver(deps.storage).may_load(key)?;
    if (curr).is_some() {
        if !curr.unwrap().is_expired(&_env.block){
            return Err(ContractError::NameTaken { name });
        }
        
    }
    let c:Config =config_read(deps.storage).load()?;
    let record = NameRecord { owner: info.sender.clone(),expiration:c.base_expiration.add(cw_utils::Duration::Height(_env.block.time.seconds())).unwrap() };
    mint_handler(&name,&info.sender,&c.cw721)?;
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::default())
}

// add reregister function so owners can extend their 

fn update_resolver( info: MessageInfo,deps: DepsMut, env: Env, name: String, new_resolver:Addr) -> Result<Response, ContractError>  {
    let c:Config =config_read(deps.storage).load()?;
    let owner_response=query_name_owner(&name,&c.cw721,&deps).unwrap();
    if owner_response.owner!=info.sender {
        return Err(ContractError::Unauthorized{});      
    }
    
    let key = name.as_bytes();
    let record = NameRecord { owner: new_resolver ,expiration:c.base_expiration.add(cw_utils::Duration::Height(env.block.time.seconds())).unwrap()};
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::default())
}
fn set_subdomain( info: MessageInfo,deps: DepsMut, env: Env, domain: String,subdomain: String, new_resolver:Addr,mint:bool,expiration:Expiration) -> Result<Response, ContractError>  {
    validate_name(&domain)?;
    validate_name(&subdomain)?;
    let c:Config =config_read(deps.storage).load()?;
    
    let owner_response=query_name_owner(&domain,&c.cw721,&deps).unwrap();
    if owner_response.owner!=info.sender {
        return Err(ContractError::Unauthorized{});      
    }
    let domain_route = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    
    if mint{
        mint_handler(&domain_route,&new_resolver,&c.cw721)?;
    }
    let record = NameRecord { owner: new_resolver,expiration:c.base_expiration.add(cw_utils::Duration::Height(env.block.time.seconds())).unwrap() };
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::default())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ResolveRecord { name } => query_resolver(deps, env, name),
        QueryMsg::Config {} => to_binary(&config_read(deps.storage).load()?),
    }
}

fn query_resolver(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
    let key = name.as_bytes();
    let address = match resolver_read(deps.storage).may_load(key)? {
        Some(record) => Some(String::from(&record.owner)),
        None => None,
    };
    let resp = ResolveRecordResponse { address };
    to_binary(&resp)
}

// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
    let is_valid = c.is_digit(10) || c.is_ascii_lowercase() || (  c == '-' || c == '_');
    !is_valid
}
fn mint_handler(name:&String,creator:&Addr,cw721:&Addr) -> StdResult<CosmosMsg>{
    let mint_msg = Cw721ExecuteMsg::Mint(MintMsg::<Extension> {
        token_id:name.to_string(),
        owner: creator.to_string(),
        token_uri:Some(String::from("test")),
        extension: None,     
    });

    let resp = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });
    Ok(resp)
}
fn burn_handler(name:String,cw721:Addr) -> StdResult<CosmosMsg>{
  
    let burn_msg: Cw721ExecuteMsg<String> = Cw721ExecuteMsg::Burn {
        token_id:name        
    };
    let resp = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&burn_msg)?,
        funds: vec![],
    });
    Ok(resp)
}
fn query_name_owner(id:&String,cw721:&Addr,deps: &DepsMut) ->Result<OwnerOfResponse,StdError>{
    let query_msg = Cw721QueryMsg::OwnerOf {
        token_id: id.clone(),
        include_expired: None,
    };
    let req=QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cw721.to_string(),
        msg: to_binary(&query_msg).unwrap(),
    });
    let res: OwnerOfResponse = deps.querier.query(&req)?;
    Ok(res)

}
/// validate_name returns an error if the name is invalid
/// (we require 3-64 lowercase ascii letters, numbers, or . - _)
fn validate_name(name: &str) -> Result<(), ContractError> {
    let length = name.len() as u64;
    if (name.len() as u64) < MIN_NAME_LENGTH {
        Err(ContractError::NameTooShort {
            length,
            min_length: MIN_NAME_LENGTH,
        })
    } else if (name.len() as u64) > MAX_NAME_LENGTH {
        Err(ContractError::NameTooLong {
            length,
            max_length: MAX_NAME_LENGTH,
        })
    } else {
        match name.find(invalid_char) {
            None => Ok(()),
            Some(bytepos_invalid_char_start) => {
                let c = name[bytepos_invalid_char_start..].chars().next().unwrap();
                Err(ContractError::InvalidCharacter { c })
            }
        }
    }
}
