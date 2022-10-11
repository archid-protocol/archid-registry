use cosmwasm_std::{
    entry_point, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, StdError, StdResult, Timestamp, Uint128, WasmMsg,
    WasmQuery,
};
use cw721::{NftInfoResponse, OwnerOfResponse};

use archid_token::{
    ExecuteMsg as Cw721ExecuteMsg, Extension, Metadata, MintMsg, QueryMsg as Cw721QueryMsg,
    UpdateMetadataMsg,
};
use cw_utils::{must_pay, Expiration};
use std::convert::TryFrom;
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MetaDataUpdateMsg, QueryMsg, RecordExpirationResponse,
    ResolveRecordResponse,
};
use crate::state::{config, config_read, mint_status, resolver, resolver_read, Config, NameRecord};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;
const MAX_BASE_INTERVAL:u64= 3;
pub type NameExtension = Option<Metadata>;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let config_state = Config {
        admin: msg.admin,
        wallet: msg.wallet,
        cw721: msg.cw721,
        base_cost: msg.base_cost,
        base_expiration: msg.base_expiration,
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
        ExecuteMsg::RenewRegistration { name } => renew_registration(deps, env, info, name),
        ExecuteMsg::UpdateResolver { name, new_resolver } => {
            update_resolver(info, deps, env, name, new_resolver)
        }
        ExecuteMsg::RegisterSubDomain {
            domain,
            subdomain,
            new_resolver,
            mint,
            expiration,
        } => set_subdomain(
            info,
            deps,
            env,
            domain,
            subdomain,
            new_resolver,
            mint,
            expiration,
        ),
        ExecuteMsg::UpdataUserDomainData {
            name,
            metadata_update,
        } => user_metadata_update_handler(info, deps, name, metadata_update),
        ExecuteMsg::UpdateConfig { update_config } => {
            _update_config(deps, env, info, update_config)
        }
        ExecuteMsg::Withdraw { amount } => withdraw_fees(info, deps, amount),
        ExecuteMsg::RemoveSubDomain{ domain,subdomain}=>remove_subdomain(info, deps,env, domain,subdomain)
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
    let curr = resolver(deps.storage).may_load(key)?;
    let c: Config = config_read(deps.storage).load()?;
    let res = must_pay(&info, &String::from("ARCH"))?;
    let mut messages = Vec::new();
    let mut registration:u64= u64::try_from(((res.checked_div(c.base_cost)).unwrap()).u128()).unwrap();
    if registration < 1 {
        return Err(ContractError::InvalidPayment { amount: res });
    }

    if registration>MAX_BASE_INTERVAL{
        registration=MAX_BASE_INTERVAL;
    }
    if (curr).is_some() {
        if !curr.unwrap().is_expired(&_env.block) {
            return Err(ContractError::NameTaken { name });
        } else {
            let burn_msg = burn_handler(&name, &c.cw721)?;
            messages.push(burn_msg);
            //&response.add_message(burn_msg);
        }
    }
    //let _expiration= c.base_expiration + _env.block.time.seconds();
    let record = NameRecord {
        owner: info.sender.clone(),
        expiration: c.base_expiration.checked_mul(registration).unwrap()+ _env.block.time.seconds(),
    };
    let mint_resp = mint_handler(
        &name,
        &info.sender,
        &c.cw721,
        c.base_expiration + _env.block.time.seconds(),
    )?;
    messages.push(mint_resp);
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::new().add_messages(messages))
}

pub fn renew_registration(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    validate_name(&name)?;
    let key = &name.as_bytes();
    let curr = (resolver(deps.storage).may_load(key)?).unwrap();
    let c: Config = config_read(deps.storage).load()?;
    if curr.is_expired(&_env.block) {
        return Err(ContractError::NameOwnershipExpired { name });
    }
    let owner_response = query_name_owner(&name, &c.cw721, &deps).unwrap();
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let record = NameRecord {
        owner: info.sender.clone(),
        expiration: c.base_expiration + curr.expiration,
    };

    let res = must_pay(&info, &String::from("ARCH"))?;
    if res != c.base_cost {
        return Err(ContractError::InvalidPayment { amount: res });
    }
    resolver(deps.storage).save(key, &record)?;
    let resp = update_metadata_expiry(deps, &c.cw721, name, c.base_expiration + curr.expiration);
    Ok(Response::new().add_message(resp.unwrap()))
}

fn update_metadata_expiry(
    deps: DepsMut,
    cw721: &Addr,
    name: String,
    expiration: u64,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    current_metadata.expiry = Some(Expiration::AtTime(Timestamp::from_seconds(expiration)));
    let resp = send_data_update(&name, &cw721, current_metadata)?;
    Ok(resp)
}
// add reregister function so owners can extend their
pub fn _update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    config_update: Config,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    if c.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    config(deps.storage).save(&config_update)?;
    Ok(Response::default())
}
fn update_resolver(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    name: String,
    new_resolver: Addr,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    let owner_response = query_name_owner(&name, &c.cw721, &deps).unwrap();
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let key = name.as_bytes();
    let curr = (resolver(deps.storage).may_load(key)?).unwrap();
    if curr.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name });
    }
    let key = name.as_bytes();
    let record = NameRecord {
        owner: new_resolver,
        expiration: curr.expiration,
    };
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::default())
}
/**
subdomain rules
only minted by domain owner
expiration<= top level domain expiration

when minted only nft owner can set subdomain resolver until expiration
nft cannot be reminted unless burned by owner before expiration
**/
fn set_subdomain(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    domain: String,
    subdomain: String,
    new_resolver: Addr,
    mint: bool,
    expiration: u64,
) -> Result<Response, ContractError> {
    validate_name(&domain)?;
    validate_name(&subdomain)?;
    let c: Config = config_read(deps.storage).load()?;
    let mut messages = Vec::new();
    let domain_route = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    let has_minted: bool = mint_status(deps.storage).may_load(key)?.is_some();
    let domain_config: NameRecord = (resolver(deps.storage).may_load(domain.as_bytes())?).unwrap();

    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
    

    if !resolver(deps.storage).may_load(&key).unwrap().is_some() {
       let metadata_msg=add_subdomain_metadata(
            &deps,
            &c.cw721,
            domain.clone(),
            subdomain.clone(),
        )?;
        messages.push(metadata_msg);
    }   

    if domain_config.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name: domain });
    }
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    // revert if minted and not expired
    if has_minted {        
        if !((resolver(deps.storage).may_load(key)?)
            .unwrap()
            .is_expired(&env.block))
        {
            return Err(ContractError::NameTaken { name: domain_route });
        }
    }
    if env.block.time >= Timestamp::from_seconds(expiration) {
        return Err(ContractError::InvalidInput {});
    }
    let _expiration = match &expiration > &domain_config.expiration {
        true => &domain_config.expiration,
        false => &expiration,
    };

    let record = NameRecord {
        owner: new_resolver.clone(),
        expiration: *_expiration,
    };
    resolver(deps.storage).save(key, &record)?;
    if mint == true {
        if !has_minted {
            mint_status(deps.storage).save(key, &true)?;
        } else {
            let burn_msg = burn_handler(&domain_route, &c.cw721)?;
            messages.push(burn_msg);
        }
        let resp = mint_handler(&domain_route, &new_resolver, &c.cw721, *_expiration)?;
        messages.push(resp);
        Ok(Response::new().add_messages(messages))
    } else {
        Ok(Response::default())
    }
}

fn remove_subdomain(info: MessageInfo,
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
fn add_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    let mut subdomains =current_metadata.subdomains.as_ref().unwrap().clone();
    subdomains.push(subdomain);
    current_metadata.subdomains=Some((*subdomains).to_vec());
    let resp = send_data_update(&name, &cw721, current_metadata)?;
    Ok(resp)
}
fn remove_subdomain_metadata(
    deps: &DepsMut,
    cw721: &Addr,
    name: String,
    subdomain: String,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    let mut subdomains =current_metadata.subdomains.as_ref().unwrap().clone();
    
    subdomains.retain(|item| &item.as_bytes() !=&subdomain.as_bytes());
    current_metadata.subdomains=Some((*subdomains).to_vec());
    let resp = send_data_update(&name, &cw721, current_metadata)?;
    Ok(resp)
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ResolveRecord { name } => query_resolver(deps, env, name),
        QueryMsg::RecordExpiration { name } => query_resolver_expiration(deps, env, name),
        QueryMsg::Config {} => to_binary(&config_read(deps.storage).load()?),
    }
}

fn query_resolver(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
    let key = name.as_bytes();
    let curr = (resolver_read(deps.storage).may_load(key)?).unwrap();

    let address = match curr.is_expired(&_env.block) {
        true => None,
        false => Some(String::from(&curr.owner)),
    };

    let resp = ResolveRecordResponse {
        address,
        expiration: curr.expiration,
    };
    to_binary(&resp)
}
fn query_resolver_expiration(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
    let key = name.as_bytes();
    let curr = (resolver_read(deps.storage).may_load(key)?).unwrap();
    let resp = RecordExpirationResponse {
        expiration: curr.expiration,
    };
    to_binary(&resp)
}
// let's not import a regexp library and just do these checks by hand
fn invalid_char(c: char) -> bool {
    let is_valid = c.is_digit(10) || c.is_ascii_lowercase() || (c == '-' || c == '_');
    !is_valid
}

fn mint_handler(
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
    let mint_msg: archid_token::ExecuteMsg = Cw721ExecuteMsg::Mint(MintMsg::<NameExtension> {
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
fn burn_handler(name: &String, cw721: &Addr) -> StdResult<CosmosMsg> {
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
fn user_metadata_update_handler(
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
fn send_tokens(to: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: (&[Coin {
            denom: String::from("ARCH"),
            amount: amount,
        }])
            .to_vec(),
    };
    Ok(msg.into())
}
fn withdraw_fees(
    info: MessageInfo,
    deps: DepsMut,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    if c.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let resp = send_tokens(&c.wallet, amount)?;
    Ok(Response::new().add_message(resp))
}

fn query_name_owner(
    id: &String,
    cw721: &Addr,
    deps: &DepsMut,
) -> Result<OwnerOfResponse, StdError> {
    let query_msg: archid_token::QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: id.clone(),
        include_expired: None,
    };
    let req = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cw721.to_string(),
        msg: to_binary(&query_msg).unwrap(),
    });
    let res: OwnerOfResponse = deps.querier.query(&req)?;
    Ok(res)
}

fn query_current_metadata(id: &String, cw721: &Addr, deps: &DepsMut) -> Result<Metadata, StdError> {
    let query_msg: archid_token::QueryMsg<Extension> = Cw721QueryMsg::NftInfo {
        token_id: id.clone(),
    };
    let req = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cw721.to_string(),
        msg: to_binary(&query_msg).unwrap(),
    });
    let res: NftInfoResponse<Metadata> = deps.querier.query(&req)?;
    Ok(res.extension)
}
fn send_data_update(name: &String, cw721: &Addr, data: Metadata) -> StdResult<CosmosMsg> {
    let update = UpdateMetadataMsg {
        token_id: name.to_string(),
        extension: Some(data),
    };
    let resp: CosmosMsg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&update)?,
        funds: vec![],
    }
    .into();
    Ok(resp)
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
