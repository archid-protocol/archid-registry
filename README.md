## Archway Nameservice

**Deployed contract_id(constantine-1 testnet)**: *archway1q5gt0quuj07yth7cnlp35p49pg22p2hyswdhpl22a2j8c0z74y0qv6yfer*


**Register name**:
```
archway tx --args '{ "register": {"name": "alex"}}'
```

**Transfer name**:
```
archway tx --args '{ "transfer": {"name": "alex", "to": "archway148tmwcuw0fsf0vk75xp9r0h26y52hfmx0nwv05"}}'
```

**Get owner address**:
```
archwayd query wasm contract-state smart archway1q5gt0quuj07yth7cnlp35p49pg22p2hyswdhpl22a2j8c0z74y0qv6yfer '{ "resolve_record": {"name": "alex" } }' --node https://rpc.constantine-1.archway.tech:443
```
