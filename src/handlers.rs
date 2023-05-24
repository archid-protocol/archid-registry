use crate::error::ContractError;

use crate::msg::MetaDataUpdateMsg;
use crate::read_utils::{
    get_subdomain_prefix, is_expired, query_current_metadata, query_name_owner, validate_name,
    validate_subdomain,
};
use crate::state::{config, config_read, resolver, Config, NameRecord, SubDomainStatus};
use crate::write_utils::{
    burn_handler, burn_remint_subdomain, mint_handler, register_new_subdomain,
    remove_subdomain_metadata, send_data_update, send_tokens, update_metadata_expiry,
    update_subdomain_expiry, update_subdomain_metadata, DENOM,
};
use archid_token::Metadata;

use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, Uint128};
use cw_utils::must_pay;
use std::convert::TryFrom;
const MAX_BASE_INTERVAL: u64 = 3;

pub fn execute_register(
    deps: DepsMut,
    env: Env,
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
    if let Some(curr_value) = curr {
        if !curr_value.is_expired(&env.block) {
            return Err(ContractError::NameTaken { name });
        } else {
            let burn_msg = burn_handler(&name, &c.cw721)?;
            messages.push(burn_msg);
        }
    }
    let expiration =
        c.base_expiration.checked_mul(registration).unwrap() + env.block.time.seconds();
    let created = env.block.time.seconds();

    let record = NameRecord {
        resolver: info.sender.clone(),
        created,
        expiration,
    };
    let mint_resp = mint_handler(&name, &info.sender, &c.cw721, created, expiration)?;
    messages.push(mint_resp);
    resolver(deps.storage).save(key, &record)?;
    let mut response = Response::new();
    response = response.add_messages(messages);
    response = response.add_attribute("action", "register");
    response = response.add_attribute("domain", name);
    Ok(response)
}

pub fn execute_renew_registration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    validate_name(&name)?;
    let key = &name.as_bytes();
    if (resolver(deps.storage).may_load(key)?).is_none() {
        return Err(ContractError::InvalidInput {});
    }
    let curr = (resolver(deps.storage).may_load(key)?).unwrap();

    let c: Config = config_read(deps.storage).load()?;
    if is_expired(&deps, key, &env.block) {
        return Err(ContractError::NameOwnershipExpired { name });
    }
    let owner_response = query_name_owner(&name, &c.cw721, &deps).unwrap();

    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let record = NameRecord {
        resolver: info.sender.clone(),
        created: env.block.time.seconds(),
        expiration: c.base_expiration + curr.expiration,
    };

    // let res = must_pay(&info, &String::from("ARCH"))?;
    let res = must_pay(&info, &String::from("uconst"))?;
    if res != c.base_cost {
        return Err(ContractError::InvalidPayment { amount: res });
    }
    resolver(deps.storage).save(key, &record)?;
    let resp = update_metadata_expiry(
        deps,
        &c.cw721,
        name.clone(),
        c.base_expiration + curr.expiration,
    );
    let mut response = Response::new();
    response = response.add_messages(resp);
    response = response.add_attribute("action", "rewew_registeration");
    response = response.add_attribute("domain", name);
    Ok(response)
}
/**
subdomain rules
only minted by domain owner
expiration<= top level domain expiration

when minted only nft owner can set subdomain resolver until expiration
nft cannot be reminted unless burned by owner before expiration
expiration for a subdomain can be extend by domain owner using
**/
#[allow(clippy::too_many_arguments)]
pub fn execute_set_subdomain(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    domain: String,
    subdomain: String,
    new_resolver: Addr,
    new_owner: Addr,
    expiration: u64,
) -> Result<Response, ContractError> {
    //
    validate_name(&domain)?;
    //
    validate_subdomain(&subdomain)?;
    //
    let c: Config = config_read(deps.storage).load()?;
    //
    let domain_route: String = format!("{}.{}", subdomain, domain);
    //
    let key = domain_route.as_bytes();
    // Check if a domain nft is currently in existence

    // check if doman resolves to a NameRecord throw error otherwise
    if resolver(deps.storage)
        .may_load(domain.as_bytes())?
        .is_none()
    {
        return Err(ContractError::InvalidInput {});
    }
    // load domain Name Record
    let domain_config: NameRecord = (resolver(deps.storage).may_load(domain.as_bytes())?).unwrap();

    // get the current name owner
    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();

    if domain_config.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name: domain });
    }
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    //set expiration to domain expiration if subdomain configuration
    let _expiration = match expiration > domain_config.expiration {
        true => &domain_config.expiration,
        false => &expiration,
    };

    let subdomain_status: SubDomainStatus;
    // add subdomain metadata to top level domain but only if hasnt been registerd
    if resolver(deps.storage).may_load(key).unwrap().is_none() {
        subdomain_status = SubDomainStatus::NewSubdomain;
    } else {
        match is_expired(&deps, key, &env.block) {
            true => subdomain_status = SubDomainStatus::ExistingMintExpired,
            false => subdomain_status = SubDomainStatus::ExistingMintActive,
        }
    }

    let messages = match subdomain_status {
        SubDomainStatus::NewSubdomain => register_new_subdomain(
            c.cw721,
            deps,
            env,
            domain.clone(),
            subdomain.clone(),
            new_resolver,
            new_owner,
            *_expiration,
        ),
        SubDomainStatus::ExistingMintExpired => burn_remint_subdomain(
            deps,
            c.cw721,
            env,
            domain.clone(),
            subdomain.clone(),
            new_resolver,
            new_owner,
            *_expiration,
        ),
        SubDomainStatus::ExistingMintActive => return Err(ContractError::Unauthorized {}),
    };
    let mut response = Response::new();
    response = response.add_messages(messages.unwrap());
    response = response.add_attribute("action", "set_subdomain");
    response = response.add_attribute("domain", domain);
    response = response.add_attribute("subdomain", subdomain);
    Ok(response)
}

pub fn execute_extend_subdomain_expiry(
    info: MessageInfo,
    deps: DepsMut,
    env: Env,
    domain: String,
    subdomain: String,
    expiration: u64,
) -> Result<Response, ContractError> {
    validate_name(&domain)?;
    //
    validate_subdomain(&subdomain)?;
    //
    let c: Config = config_read(deps.storage).load()?;
    //
    let domain_route: String = format!("{}.{}", subdomain, domain);

    // Check if a domain nft is currently in existence

    // check if doman resolves to a NameRecord throw error otherwise

    if resolver(deps.storage)
        .may_load(domain_route.as_bytes())?
        .is_none()
    {
        return Err(ContractError::InvalidInput {});
    }
    // load domain Name Record
    let domain_config: NameRecord = (resolver(deps.storage).may_load(domain.as_bytes())?).unwrap();
    let subdomain_config: NameRecord =
        (resolver(deps.storage).may_load(domain_route.as_bytes())?).unwrap();
    // get the current name owner
    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();

    if domain_config.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name: domain });
    }
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    //println!("{:?}",subdomain_config.expiration);
    //println!("{:?}",expiration);
    if expiration <= subdomain_config.expiration {
        return Err(ContractError::InvalidInput {});
    }
    let _expiration = match expiration > domain_config.expiration {
        true => domain_config.expiration,
        false => expiration,
    };
    let messages = update_subdomain_expiry(c.cw721, deps, domain, subdomain, _expiration)?;
    let mut response = Response::new();
    response = response.add_messages(messages);
    response = response.add_attribute("action", "extend_subdomain_expiry");
    response = response.add_attribute("domain", domain_route);
    Ok(response)
}

// add reregister function so owners can extend their
pub fn execute_update_config(
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
pub fn execute_update_resolver(
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
    let subdomain = get_subdomain_prefix(name.clone());
    let key = name.as_bytes();
    let curr = (resolver(deps.storage).may_load(key)?).unwrap();
    if curr.is_expired(&env.block) {
        return Err(ContractError::NameOwnershipExpired { name });
    }
    let key = name.as_bytes();
    let record = NameRecord {
        resolver: new_resolver.clone(),
        created: curr.created,
        expiration: curr.expiration,
    };
    let mut messages = Vec::new();
    if let Some(s) = subdomain {
        //let s = subdomain.unwrap();
        let resp = update_subdomain_metadata(
            &deps,
            &c.cw721,
            &s[1],
            &s[0],
            new_resolver,
            curr.expiration,
        )?;

        messages.push(resp);
    }
    resolver(deps.storage).save(key, &record)?;
    let mut response = Response::new();
    response = response.add_messages(messages);
    response = response.add_attribute("action", "update_resolver");
    response = response.add_attribute("domain", name);
    Ok(response)
}
pub fn execute_withdraw_fees(
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

pub fn execute_user_metadata_update(
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
    let mut response = Response::new();
    response = response.add_messages(resp);
    response = response.add_attribute("action", "metadata_update");
    response = response.add_attribute("domain", name);
    Ok(response)
}
pub fn execute_remove_subdomain(
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

    let owner_response = query_name_owner(&domain, &c.cw721, &deps).unwrap();
    resolver(deps.storage).remove(key);
    if owner_response.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let subdomain_owner = query_name_owner(&domain_route, &c.cw721, &deps).unwrap();
    // if owner of the minted subdomain is not owner of the top level domain
    // and subdomain is not expired
    if !is_expired(&deps, key, &env.block) && subdomain_owner.owner != info.sender {
        return Err(ContractError::NameTaken { name: domain_route });
    }
    messages.push(
        remove_subdomain_metadata(&deps, &c.cw721, domain.clone(), subdomain.clone())?,
    );
    messages.push(burn_handler(&domain_route, &c.cw721)?);
    let mut response = Response::new();
    response = response.add_messages(messages);
    response = response.add_attribute("action", "remove_subdomain");
    response = response.add_attribute("domain", domain);
    response = response.add_attribute("subdomain", subdomain);
    Ok(response)
}
