## Archway Nameservice

**Deployed contract (constantine-3)**: 

[archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r](https://testnet.mintscan.io/archway-testnet/txs/2E6CB87501E630D864DEA312D5814BF93ED4C7E87A6C8993BA640615CA016D11)


**Register name**:
```bash
# Using Archway Developer CLI
archway tx --args '{ "register": {"name": "archid"}}'
```

```bash
# Using archwayd
 archwayd tx wasm execute archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r '{"register": {"name": "archid"}}' --from keplr --chain-id "constantine-3" --node "https://rpc.constantine.archway.tech:443" --broadcast-mode sync --output json -y --gas-prices $(archwayd q rewards estimate-fees 1 --node 'https://rpc.constantine.archway.tech:443' --output json | jq -r '.gas_unit_price | (.amount + .denom)')
```

**Resolve record**:
```bash
# Using Archway Developer CLI
archway query contract-state smart --args '{"resolve_record": { "name": "archid.arch" }}'
```

```bash
# Using archwayd
archwayd query wasm contract-state smart "archway1lr8rstt40s697hqpedv2nvt27f4cuccqwvly9gnvuszxmcevrlns60xw4r" '{"resolve_record": { "name": "archid.arch" }}' --node "https://rpc.constantine.archway.tech:443"
```