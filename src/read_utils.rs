use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, QueryRequest, StdError, StdResult, WasmQuery,
};

use cw721_updatable::{NftInfoResponse, OwnerOfResponse,};
use archid_token::{
    Extension, Metadata, QueryMsg as Cw721QueryMsg,
};

use crate::error::ContractError;
use crate::msg::{ResolveRecordResponse,RecordExpirationResponse,};
use crate::state::{resolver_read,};

const MIN_NAME_LENGTH: u64 = 3;
const MAX_NAME_LENGTH: u64 = 64;

pub fn query_name_owner(
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

pub fn query_resolver(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
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
pub fn query_resolver_expiration(deps: Deps, _env: Env, name: String) -> StdResult<Binary> {
    let key = name.as_bytes();
    let curr = (resolver_read(deps.storage).may_load(key)?).unwrap();
    let resp = RecordExpirationResponse {
        expiration: curr.expiration,
    };
    to_binary(&resp)
}

pub fn query_current_metadata(id: &String, cw721: &Addr, deps: &DepsMut) -> Result<Metadata, StdError> {
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
fn invalid_char(c: char) -> bool {
    let is_valid = c.is_digit(10) || c.is_ascii_lowercase() || (c == '-' || c == '_');
    !is_valid
}
/// validate_name returns an error if the name is invalid
/// (we require 3-64 lowercase ascii letters, numbers, or . - _)
pub fn validate_name(name: &str) -> Result<(), ContractError> {
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
