## Archway Nameservice

**Deployed contract (constantine-3)**: 

[archway1fphguznhazgqdlr9mpfh6nmn3vjjr73ksz3ukznv6q7s9ndfq2csu76f3s](https://testnet.mintscan.io/archway-testnet/txs/A925D1054FD3AAE66542A482029B7B99EE67A29D8594F9DD3C5E8A0223C1B8A1)


**Register name**:
```bash
archway tx --args '{ "register": {"name": "alex"}}'
```

**Resolve record**:
```bash
 archwayd tx wasm execute archway1fphguznhazgqdlr9mpfh6nmn3vjjr73ksz3ukznv6q7s9ndfq2csu76f3s '{"register": {"name": "archid"}}' --from keplr --chain-id "constantine-3" --node "https://rpc.constantine.archway.tech:443" --broadcast-mode sync --output json -y --gas-prices $(archwayd q rewards estimate-fees 1 --node 'https://rpc.constantine.archway.tech:443' --output json | jq -r '.gas_unit_price | (.amount + .denom)')
```
