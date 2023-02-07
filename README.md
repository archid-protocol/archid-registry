## Archway Nameservice

**Deployed contract (constantine-1)**: 
[archway1nda8ud7zuzj4342vr5jxfj0fpqfwlle6cy8xgp0r5am26rdmgwrqwmdrxn](https://testnet.mintscan.io/archway-testnet/txs/F4C67C3E1EB8746902CBE8CAF9423B52C04E3CA3DCEE3DF8C3431D2EA0BD3B4B)


**Register name**:
```bash
archway tx --args '{ "register": {"name": "alex"}}'
```

**Resolve record**:
```bash
archwayd query wasm contract-state smart archway1q5gt0quuj07yth7cnlp35p49pg22p2hyswdhpl22a2j8c0z74y0qv6yfer '{ "resolve_record": {"name": "alex" } }' --node https://rpc.constantine-1.archway.tech:443
```
