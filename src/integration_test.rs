#![cfg(test)]

//use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankQuery, Coin, DepsMut, Empty, QueryRequest, StdError,
    Timestamp, Uint128, WasmMsg, WasmQuery,
};

use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info, MockStorage,
    MOCK_CONTRACT_ADDR,
};
use cw721::{
    AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, ContractInfoResponse, CustomMsg,
    Cw721Query, NftInfoResponse, NumTokensResponse, OperatorsResponse, OwnerOfResponse,
    TokensResponse,
};
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg,
    msg::QueryMsg as Cw721QueryMsg, Cw721Contract, Extension, MintMsg,
};
use cw_multi_test::{App, BankKeeper, Contract, ContractWrapper, Executor};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, RecordExpirationResponse, ResolveRecordResponse,
};
use crate::state::{Config, NameRecord};
use serde::{de::DeserializeOwned, Serialize};

fn mock_app() -> App {
    
    App::default()
}
fn get_block_time(router: &mut App) -> u64 {
    router.block_info().time.seconds()
}
fn increment_block_time(router: &mut App, new_time: u64, height_incr: u64) {
    let mut curr = router.block_info();
    curr.height = curr.height + height_incr;
    curr.time = Timestamp::from_seconds(new_time);
    router.set_block(curr);
}
pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}
pub fn contract_archID() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}
fn create_name_service(
    router: &mut App,
    _owner: Addr,
    _wallet: Addr,
    _nft: Addr,
    _base_cost: u64,
    _base_expiration: u64,
) -> Addr {
    let contract_id = router.store_code(contract_archID());
    //let owner = _owner.clone().to_string();
    let msg = InstantiateMsg {
        admin: _owner.clone(),
        wallet: _wallet,
        cw721: _nft,
        base_cost: _base_cost,
        base_expiration: _base_expiration,
    };
    let name_addr = router
        .instantiate_contract(contract_id, _owner, &msg, &[], "archID", None)
        .unwrap();
    name_addr
}
fn create_cw721(router: &mut App, minter: &Addr) -> Addr {
    //let contract = Cw721Contract::default();
    let cw721_id = router.store_code(contract_cw721());
    let msg = Cw721InstantiateMsg {
        name: "TESTNFT".to_string(),
        symbol: "TSNFT".to_string(),
        minter: String::from(minter),
    };
    let contract = router
        .instantiate_contract(cw721_id, minter.clone(), &msg, &[], "swap721", None)
        .unwrap();
    contract
}
pub fn query<M, T>(router: &mut App, target_contract: Addr, msg: M) -> Result<T, StdError>
where
    M: Serialize + DeserializeOwned,
    T: Serialize + DeserializeOwned,
{
    router.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: target_contract.to_string(),
        msg: to_binary(&msg).unwrap(),
    }))
}

#[test]
fn test_domains() {
    let mut app = mock_app();
    let current_time = get_block_time(&mut app);

    increment_block_time(&mut app, current_time + 1000, 7);

    assert_eq!(get_block_time(&mut app), current_time + 1000);
    let owner = Addr::unchecked("owner");
    let wallet = Addr::unchecked("wallet");
    let name_owner = Addr::unchecked("mintnames");
    let mock = Addr::unchecked("testtesttest");
    let domain_owner = Addr::unchecked("domain_owner");

    let name_service = create_name_service(
        &mut app,
        owner.clone(),
        wallet.clone(),
        mock.clone(),
        5000,
        10000000,
    );
    let nft = create_cw721(&mut app, &name_service);
    let update_config = Config {
        admin: owner.clone(),
        wallet: wallet.clone(),
        cw721: nft.clone(),
        base_cost: 0,
        base_expiration: 86400,
    };
    let update_msg = ExecuteMsg::UpdateConfig {
        update_config: update_config,
    };
    /**
    app
        .execute_contract(owner.clone(), nft.clone(), &mint_msg, &[])
        .unwrap();
    **/
    app.execute_contract(owner.clone(), name_service.clone(), &update_msg, &[]);

    let info: Config = query(&mut app, name_service.clone(), QueryMsg::Config {}).unwrap();
    let register_msg = ExecuteMsg::Register {
        name: String::from("simpletest"),
    };
    let res = app.execute_contract(name_owner.clone(), name_service.clone(), &register_msg, &[]);
    println!("{:?}", res);
    let owner_query: Cw721QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: String::from("simpletest"),
        include_expired: None,
    };
    let total: NumTokensResponse = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NumTokens {},
    )
    .unwrap();
    println!("{}", total.count);

    let resolve: ResolveRecordResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::ResolveRecord {
            name: String::from("simpletest"),
        },
    )
    .unwrap();
    let nft_owner: OwnerOfResponse = query(&mut app, nft.clone(), owner_query).unwrap();
    println!("{:?}", resolve.address.unwrap());
    println!("{:?}", nft_owner);
    let expiration: RecordExpirationResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::RecordExpiration {
            name: String::from("simpletest"),
        },
    )
    .unwrap();
    println!("{:?}", expiration);
    let subdomain_msg = ExecuteMsg::RegisterSubDomain {
        domain: String::from("simpletest"),
        subdomain: String::from("subdomain"),
        new_resolver: mock.clone(),
        mint: false,
        expiration: expiration.expiration
    };
    let res2 = app.execute_contract(name_owner.clone(), name_service.clone(), &subdomain_msg, &[]);
    println!("{:?}", res2);
    let subresolve: ResolveRecordResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::ResolveRecord {
            name: String::from("subdomain.simpletest"),
        },
    ).unwrap();
    println!("{:?}", subresolve);
}
