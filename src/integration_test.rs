#![cfg(test)]
use cosmwasm_std::{
    to_binary, Addr, Coin, Empty, QueryRequest, StdError, Timestamp, Uint128, WasmQuery,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use serde::{de::DeserializeOwned, Serialize};

use archid_token::{
    Extension, InstantiateMsg as Cw721InstantiateMsg, Metadata, QueryMsg as Cw721QueryMsg,
};
use cw721_updatable::{NftInfoResponse, NumTokensResponse, OwnerOfResponse};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, RecordExpirationResponse, ResolveRecordResponse,
};
use crate::state::Config;
use crate::write_utils::DENOM;

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
        archid_token::entry::execute,
        archid_token::entry::instantiate,
        archid_token::entry::query,
    );
    Box::new(contract)
}
pub fn contract_archid() -> Box<dyn Contract<Empty>> {
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
    _base_cost: Uint128,
    _base_expiration: u64,
) -> Addr {
    let contract_id = router.store_code(contract_archid());
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
fn mint_native(app: &mut App, beneficiary: String, denom: String, amount: Uint128) {
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: beneficiary,
            amount: vec![Coin {
                denom: denom,
                amount: amount,
            }],
        },
    ))
    .unwrap();
}

//test setup domain minting and subdomain minting
#[test]
fn basic_domain_test() {
    let mut app = mock_app();
    let current_time = get_block_time(&mut app);

    increment_block_time(&mut app, current_time + 1000, 7);

    assert_eq!(get_block_time(&mut app), current_time + 1000);
    let owner = Addr::unchecked("owner");
    let wallet = Addr::unchecked("wallet");
    let name_owner = Addr::unchecked("mintnames");
    let mock = Addr::unchecked("testtesttest");
    let _domain_owner = Addr::unchecked("domain_owner");
    mint_native(
        &mut app,
        name_owner.to_string(),
        String::from(DENOM),
        Uint128::from(10000u128),
    );
    let name_service = create_name_service(
        &mut app,
        owner.clone(),
        wallet.clone(),
        mock.clone(),
        Uint128::from(5000u64),
        10000000,
    );
    let nft = create_cw721(&mut app, &name_service);
    let update_config = Config {
        admin: owner.clone(),
        wallet: wallet.clone(),
        cw721: nft.clone(),
        base_cost: Uint128::from(5000u64),
        base_expiration: 86400,
    };
    let update_msg = ExecuteMsg::UpdateConfig {
        config: update_config,
    };
    print!("{}","Starting QUERY");
  
    let _config_update =
        app.execute_contract(owner.clone(), name_service.clone(), &update_msg, &[]);

    let _info: Config = query(&mut app, name_service.clone(), QueryMsg::Config {}).unwrap();
    let register_msg = ExecuteMsg::Register {
        name: String::from("simpletest"),
    };
    assert!(app
        .execute_contract(name_owner.clone(), name_service.clone(), &register_msg, &[])
        .is_err());

    let _res = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &register_msg,
        &[Coin {
            denom: String::from(DENOM),
            amount: Uint128::from(5000u128),
        }],
    );
    assert!(app
        .execute_contract(
            name_owner.clone(),
            name_service.clone(),
            &register_msg,
            &[Coin {
                denom: String::from(DENOM),
                amount: Uint128::from(5000u128)
            }]
        )
        .is_err());

    // println!("{:?}", res);
    let owner_query: Cw721QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: String::from("simpletest.arch"),
        include_expired: None,
    };
    let _owner_query2: Cw721QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: String::from("lolz.arch"),
        include_expired: None,
    };
    print!("{}","Starting QUERY");
    let _total: NumTokensResponse = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NumTokens {},
    )
    .unwrap();
     println!("{}", _total.count);

    let _resolve: ResolveRecordResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::ResolveRecord {
            name: String::from("simpletest.arch"),
        },
    )
    .unwrap();
    let _nft_owner: OwnerOfResponse = query(&mut app, nft.clone(), owner_query).unwrap();
    println!("{:?}", _nft_owner);
    // println!("{:?}", resolve.address.unwrap());
    // println!("{:?}", nft_owner);

    let expiration: RecordExpirationResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::RecordExpiration {
            name: String::from("simpletest.arch"),
        },
    )
    .unwrap();
    println!("{:?}", expiration);
    let subdomain_msg = ExecuteMsg::RegisterSubDomain {
        domain: String::from("simpletest"),
        subdomain: String::from("dapp"),
        new_resolver: mock.clone(),
        new_owner: mock.clone(),
        mint: true,
        expiration: expiration.expiration,
    };
    let _res2 = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &subdomain_msg,
        &[],
    );
    println!("First subdomain execute {:?}", _res2);
    let _subresolve: ResolveRecordResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::ResolveRecord {
            name: String::from("dapp.simpletest.arch"),
        },
    )
    .unwrap();
    // println!("First subdomain resolver query {:?}", subresolve);

    let subdomain_cw721: NftInfoResponse<Extension> = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NftInfo {
            token_id: String::from("dapp.simpletest.arch"),
        },
    )
    .unwrap();

    let metadata_extension: Extension = Some(Metadata {
        name: Some("dapp.simpletest".into()),
        description: Some("dapp.simpletest.arch subdomain".into()),
        image: None,
        expiry: Some(expiration.expiration),
        domain: Some("dapp.simpletest.arch".into()),
        subdomains: None,
        accounts: None,
        websites: None,
    });
    assert_eq!(
        subdomain_cw721,
        NftInfoResponse::<Extension> {
            token_uri: None,
            extension: metadata_extension,
        }
    );
    // println!("First subdomain NFT metadata query {:?}", subdomain_cw721);

    let subdomain_msg2 = ExecuteMsg::RegisterSubDomain {
        domain: String::from("simpletest"),
        subdomain: String::from("subdomain2"),
        new_resolver: mock.clone(),
        new_owner: mock.clone(),
        mint: true,
        expiration: expiration.expiration,
    };
    let _res3 = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &subdomain_msg2,
        &[],
    );
    // println!("Second subdomain execute {:?}", res3);

    let subdomain2_cw721: NftInfoResponse<Extension> = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NftInfo {
            token_id: String::from("subdomain2.simpletest.arch"),
        },
    )
    .unwrap();
    // println!("Second subdomain metadata query {:?}", subdomain2_cw721);

    let metadata_extension2: Extension = Some(Metadata {
        name: Some("subdomain2.simpletest".into()),
        description: Some("subdomain2.simpletest.arch subdomain".into()),
        image: None,
        expiry: Some(expiration.expiration),
        domain: Some("subdomain2.simpletest.arch".into()),
        subdomains: None,
        accounts: None,
        websites: None,
    });
    assert_eq!(
        subdomain2_cw721,
        NftInfoResponse::<Extension> {
            token_uri: None,
            extension: metadata_extension2,
        }
    );

    let total2: NumTokensResponse = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NumTokens {},
    )
    .unwrap();
    assert_eq!(total2.count, 3);
    // dbg!(total2.count);
}

#[test]
fn test_expired_domains() {
    let mut app = mock_app();
    let mut current_time = get_block_time(&mut app);
    let owner = Addr::unchecked("owner");
    let wallet = Addr::unchecked("wallet");
    let name_owner = Addr::unchecked("mintnames");
    let name_owner2 = Addr::unchecked("mintothernames");
    let mock = Addr::unchecked("testtesttest");
    let _domain_owner = Addr::unchecked("domain_owner");
    mint_native(
        &mut app,
        name_owner.to_string(),
        String::from(DENOM),
        Uint128::from(10000u128),
    );
    mint_native(
        &mut app,
        name_owner2.to_string(),
        String::from(DENOM),
        Uint128::from(10000u128),
    );
    let name_service = create_name_service(
        &mut app,
        owner.clone(),
        wallet.clone(),
        mock.clone(),
        Uint128::from(5000u64),
        10000000,
    );
    let nft = create_cw721(&mut app, &name_service);
    let update_config = Config {
        admin: owner.clone(),
        wallet: wallet.clone(),
        cw721: nft.clone(),
        base_cost: Uint128::from(5000u64),
        base_expiration: 86400,
    };
    let update_msg = ExecuteMsg::UpdateConfig {
        config: update_config,
    };
    let _config_update =
        app.execute_contract(owner.clone(), name_service.clone(), &update_msg, &[]);

    let _info: Config = query(&mut app, name_service.clone(), QueryMsg::Config {}).unwrap();
    let register_msg = ExecuteMsg::Register {
        name: String::from("simpletest"),
    };
    let _register = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &register_msg,
        &[Coin {
            denom: String::from(DENOM),
            amount: Uint128::from(5000u128),
        }],
    );
    increment_block_time(&mut app, current_time + 86401, 777);
    let resolve: ResolveRecordResponse = query(
        &mut app,
        name_service.clone(),
        QueryMsg::ResolveRecord {
            name: String::from("simpletest.arch"),
        },
    )
    .unwrap();
    assert!(resolve.address == None);

    current_time = get_block_time(&mut app);

    let subdomain_msg = ExecuteMsg::RegisterSubDomain {
        domain: String::from("simpletest"),
        subdomain: String::from("subdomain"),
        new_resolver: mock.clone(),
        new_owner: mock.clone(),
        mint: false,
        expiration: current_time + 1000,
    };
    assert!(app
        .execute_contract(
            name_owner.clone(),
            name_service.clone(),
            &subdomain_msg,
            &[]
        )
        .is_err());

    // assert_eq!(
    //         info1,
    //         NftInfoResponse {
    //             token_uri: None,
    //             extension: metadata_extension.clone(),
    //         }
    // );
    //println!("{:?}", info1);
    let _transfer = app.execute_contract(
        name_owner2.clone(),
        name_service.clone(),
        &register_msg,
        &[Coin {
            denom: String::from(DENOM),
            amount: Uint128::from(5000u128),
        }],
    );
    let owner_query: Cw721QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: String::from("simpletest.arch"),
        include_expired: None,
    };

    let nft_owner: OwnerOfResponse = query(&mut app, nft.clone(), owner_query).unwrap();
    assert!(nft_owner.owner == name_owner2);

    let info1: NftInfoResponse<Extension> = query(
        &mut app,
        nft.clone(),
        Cw721QueryMsg::<Extension>::NftInfo {
            token_id: String::from("simpletest.arch"),
        },
    )
    .unwrap();

    let expiry_resp = info1.clone().extension.as_ref().unwrap().expiry;
    let metadata_extension: Extension = Some(Metadata {
        name: Some("simpletest".into()),
        description: Some("simpletest.arch domain".into()),
        image: None,
        expiry: expiry_resp,
        domain: Some("simpletest.arch".into()),
        subdomains: Some(vec![]),
        accounts: Some(vec![]),
        websites: Some(vec![]),
    });
    assert_eq!(
        info1,
        NftInfoResponse::<Extension> {
            token_uri: None,
            extension: metadata_extension,
        }
    );
    // println!("simpletest.arch metadata {:?}", info1);
}

#[test]
fn test_subdomain_rules() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let wallet = Addr::unchecked("wallet");
    let name_owner = Addr::unchecked("mintnames");
    let name_owner2 = Addr::unchecked("mintothernames");
    let mock = Addr::unchecked("testtesttest");
    let domain_owner = Addr::unchecked("domain_owner");
    mint_native(
        &mut app,
        name_owner.to_string(),
        String::from(DENOM),
        Uint128::from(10000u128),
    );
    mint_native(
        &mut app,
        name_owner2.to_string(),
        String::from(DENOM),
        Uint128::from(10000u128),
    );
    let name_service = create_name_service(
        &mut app,
        owner.clone(),
        wallet.clone(),
        mock.clone(),
        Uint128::from(5000u64),
        10000000,
    );
    let nft = create_cw721(&mut app, &name_service);
    let update_config = Config {
        admin: owner.clone(),
        wallet: wallet.clone(),
        cw721: nft.clone(),
        base_cost: Uint128::from(5000u64),
        base_expiration: 86400,
    };
    let update_msg = ExecuteMsg::UpdateConfig {
        config: update_config,
    };
    let _config_update =
        app.execute_contract(owner.clone(), name_service.clone(), &update_msg, &[]);

    let _info: Config = query(&mut app, name_service.clone(), QueryMsg::Config {}).unwrap();
    let register_msg = ExecuteMsg::Register {
        name: String::from("simpletest"),
    };
    let _transfer = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &register_msg,
        &[Coin {
            denom: String::from(DENOM),
            amount: Uint128::from(5000u128),
        }],
    );
    let mut current_time = get_block_time(&mut app);
    let subdomain_msg = ExecuteMsg::RegisterSubDomain {
        domain: String::from("simpletest"),
        subdomain: String::from("subdomain"),
        new_resolver: name_owner2.clone(),
        new_owner: mock.clone(),
        mint: true,
        expiration: current_time + 43200,
    };
    let _res3 = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &subdomain_msg,
        &[],
    );
    // println!("{:?}", res3);
    //app.execute_contract(name_owner.clone(), name_service.clone(), &subdomain_msg, &[]);
    assert!(app
        .execute_contract(
            name_owner.clone(),
            name_service.clone(),
            &subdomain_msg,
            &[]
        )
        .is_err());
    let update_resolver_msg = ExecuteMsg::UpdateResolver {
        name: String::from("subdomain.simpletest"),
        new_resolver: domain_owner,
    };
    assert!(app
        .execute_contract(
            name_owner.clone(),
            name_service.clone(),
            &update_resolver_msg,
            &[]
        )
        .is_err());
    let _ = app.execute_contract(
        name_owner2.clone(),
        name_service.clone(),
        &update_resolver_msg,
        &[],
    );
    current_time = get_block_time(&mut app);
    increment_block_time(&mut app, current_time + 43205, 77);
    let _res4 = app.execute_contract(
        name_owner.clone(),
        name_service.clone(),
        &subdomain_msg,
        &[],
    );
    let owner_query: Cw721QueryMsg<Extension> = Cw721QueryMsg::OwnerOf {
        token_id: String::from("simpletest.arch"),
        include_expired: None,
    };

    let nft_owner: OwnerOfResponse = query(&mut app, nft.clone(), owner_query).unwrap();
    assert!(nft_owner.owner == name_owner);
}
