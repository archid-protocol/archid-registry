## Archway Nameservice

**Deployed contract_id(constantine-1 testnet)**: *archway1nda8ud7zuzj4342vr5jxfj0fpqfwlle6cy8xgp0r5am26rdmgwrqwmdrxn*


**Register name**:
```
archway tx --args '{ "register": {"name": "alex"}}'
```

**Resolve record**:
```
archwayd query wasm contract-state smart archway1q5gt0quuj07yth7cnlp35p49pg22p2hyswdhpl22a2j8c0z74y0qv6yfer '{ "resolve_record": {"name": "alex" } }' --node https://rpc.constantine-1.archway.tech:443
```
