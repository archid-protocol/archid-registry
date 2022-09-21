use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    QueryRequest, Response, StdError, StdResult, WasmMsg, WasmQuery,
};
use cw721::OwnerOfResponse;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::QueryMsg as Cw721QueryMsg, Extension, MintMsg,
};
use cw_utils::{must_pay, Duration, Expiration};
use std::ops::Add;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, RecordExpirationResponse, ResolveRecordResponse,
};
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
        ExecuteMsg::UpdateConfig { update_config } => {
            _update_config(deps, env, info, update_config)
        }
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
    //must_pay(&info, &c.base_cost.to_string())?;
    if (curr).is_some() {
        if !curr.unwrap().is_expired(&_env.block) {
            return Err(ContractError::NameTaken { name });
        }
    }

    let record = NameRecord {
        owner: info.sender.clone(),
        expiration: c.base_expiration + _env.block.time.seconds(),
    };
    let resp = mint_handler(&name, &info.sender, &c.cw721)?;

    resolver(deps.storage).save(key, &record)?;
    Ok(Response::new().add_message(resp))
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

    must_pay(&info, &c.base_cost.to_string())?;
    resolver(deps.storage).save(key, &record)?;
    Ok(Response::default())
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

    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let domain_route = format!("{}.{}", subdomain, domain);
    let key = domain_route.as_bytes();
    if mint {
        mint_handler(&domain_route, &new_resolver, &c.cw721)?;
    }
    
    let curr = resolver(deps.storage).may_load(key)?;
    let record = NameRecord {
        owner: new_resolver,
        expiration: expiration,
    };
    resolver(deps.storage).save(key, &record)?;
    
    Ok(Response::default())
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

    let address = match !curr.is_expired(&_env.block) {
        true => Some(String::from(&curr.owner)),
        false => None,
    };

    let resp = ResolveRecordResponse { address };
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
fn mint_handler(name: &String, creator: &Addr, cw721: &Addr) -> StdResult<CosmosMsg> {
    let mint_msg: cw721_base::ExecuteMsg<Extension, Extension> =
        Cw721ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: name.to_string(),
            owner: creator.to_string(),
            token_uri: Some(String::from("test")),
            extension: None,
        });

    let resp: CosmosMsg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }
    .into();
    Ok(resp)
}
/*fn send_tokens(to: &Addr, amount: Balance) -> StdResult<Vec<SubMsg>> {
    if amount.is_empty() {
        Ok(vec![])
    } else {
        let msg = BankMsg::Send {
            to_address: to.into(),
            amount: Coin::new(amount,'ARCH'),
        };
        Ok(vec![SubMsg::new(msg)])
    }
}
fn withdrawFees(info: MessageInfo, deps: DepsMut, env: Env, amount: u64) -> StdResult<CosmosMsg> {
    let c: Config = config_read(deps.storage).load()?;
    if c.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    send_tokens(c.wallet, msg.amount);
}
*/
fn burn_handler(name: String, cw721: Addr) -> StdResult<CosmosMsg> {
    let burn_msg: Cw721ExecuteMsg<Empty, Extension> = Cw721ExecuteMsg::Burn { token_id: name };
    let resp = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_binary(&burn_msg)?,
        funds: vec![],
    });
    Ok(resp)
}
fn query_name_owner(
    id: &String,
    cw721: &Addr,
    deps: &DepsMut,
) -> Result<OwnerOfResponse, StdError> {
    let query_msg: cw721_base::QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
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
