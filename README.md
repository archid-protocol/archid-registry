## ArchID Name Service for Archway Network

**Deployed contract (constantine-3)**: 

[archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r](https://testnet.mintscan.io/archway-testnet/txs/2E6CB87501E630D864DEA312D5814BF93ED4C7E87A6C8993BA640615CA016D11)


**Instantiate Registry contract**:
```bash
# Using Archway Developer CLI
archway instantiate --args '{ "admin": "archway1f395p0gg67mmfd5zcqvpnp9cxnu0hg6r9hfczq", "wallet": "archway1f395p0gg67mmfd5zcqvpnp9cxnu0hg6r9hfczq", "cw721": "tbd", "base_cost": "250000000000000000", "base_expiration": 31536000 }'
```

**Configure Registry contract**:
```bash
# Using archwayd (e.g. add correct "cw721" value)
archwayd tx wasm execute archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r '{ "update_config": { "config": { "admin": "archway1f395p0gg67mmfd5zcqvpnp9cxnu0hg6r9hfczq", "wallet": "archway1f395p0gg67mmfd5zcqvpnp9cxnu0hg6r9hfczq", "cw721": "archway146htsfvftmq8fl26977w9xgdwmsptr2quuf7yyra4j0gttx32z3secq008", "base_cost": "250000000000000000", "base_expiration": 31536000 } } }' --from keplr --chain-id "constantine-3" --node "https://rpc.constantine.archway.tech:443" --broadcast-mode sync --output json -y --gas-prices $(archwayd q rewards estimate-fees 1 --node 'https://rpc.constantine.archway.tech:443' --output json | jq -r '.gas_unit_price | (.amount + .denom)')
```


**Register a domain**:
```bash
# Using Archway Developer CLI
archway tx --args '{ "register": {"name": "archid"}}'
```

```bash
# Using archwayd
 archwayd tx wasm execute archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r '{"register": {"name": "archid"}}' --from keplr --chain-id "constantine-3" --node "https://rpc.constantine.archway.tech:443" --broadcast-mode sync --output json -y --gas-prices $(archwayd q rewards estimate-fees 1 --node 'https://rpc.constantine.archway.tech:443' --output json | jq -r '.gas_unit_price | (.amount + .denom)')
```

**Resolve a record**:
```bash
# Using Archway Developer CLI
archway query contract-state smart --args '{"resolve_record": { "name": "archid.arch" }}'
```

```bash
# Using archwayd
archwayd query wasm contract-state smart "archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r" '{"resolve_record": { "name": "archid.arch" }}' --node "https://rpc.constantine.archway.tech:443"
```