use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Timestamp, Uint128,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::read_utils::{
    format_name, query_current_metadata, query_name_owner, query_resolver,
    query_resolver_expiration, validate_name, validate_subdomain,
};
use crate::state::{config, config_read, mint_status, resolver, Config, NameRecord};
use crate::write_utils::{
    add_subdomain_metadata, burn_handler, mint_handler, remove_subdomain, send_data_update,
    send_tokens, user_metadata_update_handler, DENOM,
};
use archid_token::Metadata;
use cw_utils::must_pay;
use std::convert::TryFrom;

const MAX_BASE_INTERVAL: u64 = 3;
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
        ExecuteMsg::Register { name } => execute_register(deps, env, info, format_name(name)),
        ExecuteMsg::RenewRegistration { name } => {
            renew_registration(deps, env, info, format_name(name))
        }
        ExecuteMsg::UpdateResolver { name, new_resolver } => {
            update_resolver(info, deps, env, format_name(name), new_resolver)
        }
        ExecuteMsg::RegisterSubDomain {
            domain,
            subdomain,
            new_resolver,
            new_owner,
            mint,
            expiration,
        } => set_subdomain(
            info,
            deps,
            env,
            format_name(domain),
            subdomain,
            new_resolver,
            new_owner,
            mint,
            expiration,
        ),
        ExecuteMsg::UpdataUserDomainData {
            name,
            metadata_update,
        } => user_metadata_update_handler(info, deps, format_name(name), metadata_update),
        ExecuteMsg::UpdateConfig { update_config } => {
            _update_config(deps, env, info, update_config)
        }
        ExecuteMsg::Withdraw { amount } => withdraw_fees(info, deps, amount),
        ExecuteMsg::RemoveSubDomain { domain, subdomain } => {
            remove_subdomain(info, deps, env, format_name(domain), subdomain)
        }
    }
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ResolveRecord { name } => query_resolver(deps, env, name),
        QueryMsg::RecordExpiration { name } => query_resolver_expiration(deps, env, name),
        QueryMsg::Config {} => to_binary(&config_read(deps.storage).load()?),
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
    let res = must_pay(&info, &String::from(DENOM))?;
    let mut messages = Vec::new();
    let mut registration: u64 =
        u64::try_from(((res.checked_div(c.base_cost)).unwrap()).u128()).unwrap();
    if registration < 1 {
        return Err(ContractError::InvalidPayment { amount: res });
    }

    if registration > MAX_BASE_INTERVAL {
        registration = MAX_BASE_INTERVAL;
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
        expiration: c.base_expiration.checked_mul(registration).unwrap()
            + _env.block.time.seconds(),
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

    // let res = must_pay(&info, &String::from("ARCH"))?;
    let res = must_pay(&info, &String::from("uconst"))?;
    if res != c.base_cost {
        return Err(ContractError::InvalidPayment { amount: res });
    }
    resolver(deps.storage).save(key, &record)?;
    let resp = update_metadata_expiry(deps, &c.cw721, name, c.base_expiration + curr.expiration);
    Ok(Response::new().add_message(resp.unwrap()))
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
    new_owner: Addr,
    mint: bool,
    expiration: u64,
) -> Result<Response, ContractError> {
    validate_name(&domain)?;
    validate_subdomain(&subdomain)?;
    let c: Config = config_read(deps.storage).load()?;
    let mut messages = Vec::new();
    let domain_route: String = format!("{}.{}", subdomain, domain);
    //println!("{}",domain_route);
    let key = domain_route.as_bytes();
    let has_minted: bool = mint_status(deps.storage).may_load(key)?.is_some();
    let domain_config: NameRecord = (resolver(deps.storage).may_load(domain.as_bytes())?).unwrap();

    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
    let _expiration = match &expiration > &domain_config.expiration {
        true => &domain_config.expiration,
        false => &expiration,
    };

    if !resolver(deps.storage).may_load(&key).unwrap().is_some() {
        let metadata_msg = add_subdomain_metadata(
            &deps,
            &c.cw721,
            domain.clone(),
            subdomain.clone(),
            new_resolver.clone(),
            _expiration.clone(),
            mint,
        )?;
        messages.push(metadata_msg);
    }
    if has_minted {
        if !((resolver(deps.storage).may_load(key)?)
            .unwrap()
            .is_expired(&env.block))
        {
            return Err(ContractError::NameTaken { name: domain_route });
        }
    }
    if domain_config.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name: domain });
    }
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    // revert if minted and not expired

    if env.block.time >= Timestamp::from_seconds(expiration) {
        return Err(ContractError::InvalidInput {});
    }

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
        let resp = mint_handler(&domain_route, &new_owner, &c.cw721, *_expiration)?;
        messages.push(resp);
        Ok(Response::new().add_messages(messages))
    } else {
        Ok(Response::default())
    }
}

fn update_metadata_expiry(
    deps: DepsMut,
    cw721: &Addr,
    name: String,
    expiration: u64,
) -> StdResult<CosmosMsg> {
    let mut current_metadata: Metadata = query_current_metadata(&name, &cw721, &deps).unwrap();
    current_metadata.expiry = Some(expiration);
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
pub fn update_resolver(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    name: String,
    new_resolver: Addr,
) -> Result<Response, ContractError> {
    let c: Config = config_read(deps.storage).load()?;
    let owner_response = query_name_owner(&name, &c.cw721, &deps)?;
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
pub fn withdraw_fees(
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
