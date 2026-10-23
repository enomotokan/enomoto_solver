# Local-MIP 試作の計測 s4 (2026-10-23)

- ソルバー: f970f87 (inv/heur-ls-20261023 の 1701b36 でビルド)、4 スレッド・60 秒、`scripts/bench_cand_20261020.py`
- 担当: gr4x6 dcmulti fiber nw04 bppc4-08 neos-950242 mzzv42z
- 設定: `newdef` = 既定、`mix` = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、`mixrole` = `mix` + `ENOMOTO_MIP_ROLE_LMIP=1`
- 順番: newdef HLa → mix HLa → mixrole HLa → mixrole HLb → mix HLb → newdef HLb
- 結果: `benchmarks/*_f970f87_{newdef,mix,mixrole}_s4_rHL{a,b}.{json,md}`、`benchmarks/cand_logs/*_s4_rHL*.log`

## 表 (時間 [s]、最初の解 = 時刻 [s] / 値)

| 問題 | 回 | 状態 | 目的値 | 時間 | 最初の解 | ノード |
|---|---|---|---:|---:|---|---:|
| gr4x6 | newdef HLa | optimal | 202.35 | 0.04 | 0.032 / 229.05 | 0 |
| | newdef HLb | optimal | 202.35 | 0.03 | 0.015 / 229.05 | 0 |
| | mix HLa | optimal | 202.35 | 0.02 | 0.004 / 229.05 | 0 |
| | mix HLb | optimal | 202.35 | 0.05 | 0.015 / 229.05 | 0 |
| | mixrole HLa | optimal | 202.35 | 0.02 | 0.005 / 229.05 | 0 |
| | mixrole HLb | optimal | 202.35 | 0.03 | 0.010 / 229.05 | 0 |
| dcmulti | newdef HLa | optimal | 188182 | 1.01 | 0.414 / 194485.1 | 4 |
| | newdef HLb | optimal | 188182 | 1.29 | 0.442 / 194485.1 | 4 |
| | mix HLa | optimal | 188182 | 1.01 | 0.112 / 211471.7 | 4 |
| | mix HLb | optimal | 188182 | 1.02 | 0.093 / 211471.7 | 4 |
| | mixrole HLa | optimal | 188182 | 1.05 | 0.092 / 211471.7 | 4 |
| | mixrole HLb | optimal | 188182 | 1.07 | 0.120 / 211471.7 | 4 |
| fiber | newdef HLa | optimal | 405935.18 | 1.69 | 0.447 / 614431.83 | 3 |
| | newdef HLb | optimal | 405935.18 | 1.61 | 0.452 / 614431.83 | 3 |
| | mix HLa | optimal | 405935.18 | 1.35 | 0.114 / 1255418.51 | 1 |
| | mix HLb | optimal | 405935.18 | 1.18 | 0.130 / 1255418.51 | 5 |
| | mixrole HLa | optimal | 405935.18 | 1.65 | 0.129 / 1255418.51 | 3 |
| | mixrole HLb | optimal | 405935.18 | 1.28 | 0.148 / 1255418.51 | 3 |
| nw04 | newdef HLa | optimal | 16862 | 7.84 | 7.683 / 52354 | 0 |
| | newdef HLb | optimal | 16862 | 7.50 | 7.397 / 52354 | 0 |
| | mix HLa | optimal | 16862 | 7.69 | 7.582 / 53592 | 0 |
| | mix HLb | optimal | 16862 | 8.26 | 8.121 / 53592 | 0 |
| | mixrole HLa | optimal | 16862 | 7.62 | 7.526 / 53592 | 0 |
| | mixrole HLb | optimal | 16862 | 7.69 | 7.502 / 53592 | 0 |
| bppc4-08 | newdef HLa | time_limit | 54 | 60.04 | 1.716 / 94 | 23028 |
| | newdef HLb | time_limit | 54 | 60.04 | 1.667 / 94 | 22915 |
| | mix HLa | time_limit | 54 | 60.08 | 0.670 / 108 | 21139 |
| | mix HLb | time_limit | 54 | 60.04 | 0.707 / 108 | 24687 |
| | mixrole HLa | time_limit | 54 | 60.05 | 0.564 / 108 | 20667 |
| | mixrole HLb | time_limit | 54 | 60.05 | 0.787 / 108 | 24167 |
| neos-950242 | newdef HLa | optimal | 4 | 13.32 | 8.138 / 19 | 1 |
| | newdef HLb | optimal | 4 | 22.79 | 8.332 / 19 | 1 |
| | mix HLa | optimal | 4 | 8.42 | 8.312 / 4 | 0 |
| | mix HLb | optimal | 4 | 8.50 | 8.394 / 4 | 0 |
| | mixrole HLa | optimal | 4 | 9.08 | 8.934 / 4 | 0 |
| | mixrole HLb | optimal | 4 | 9.20 | 9.065 / 4 | 0 |
| mzzv42z | newdef HLa | time_limit | -20220 | 60.04 | 9.918 / -990 | 0 |
| | newdef HLb | time_limit | -20470 | 60.07 | 8.661 / -990 | 0 |
| | mix HLa | time_limit | -20240 | 62.04 | 7.734 / -3620 | 74 |
| | mix HLb | time_limit | -20230 | 60.04 | 6.957 / -3620 | 0 |
| | mixrole HLa | time_limit | -20170 | 60.06 | 8.002 / -3620 | 0 |
| | mixrole HLb | time_limit | -19442 | 62.05 | 7.367 / -3620 | 47 |

HiGHS (参考): bppc4-08 は 60 秒で 53 (時間切れ)、mzzv42z は最適 -20540、他はすべて同じ最適値。

## 所見

- 目的値の不一致・異常終了はなし (解けた問題はすべて HiGHS と同じ最適値)。
- neos-950242: mix・mixrole は根で最適値 4 の解を最初に見つけ (newdef は 19)、ノード 0 で終わる。時間は newdef 13.3 / 22.8 s → mix 8.4 / 8.5 s、mixrole 9.1 / 9.2 s (2 組とも短い)。
- dcmulti・fiber・bppc4-08: mix・mixrole は最初の解が早い (dcmulti 0.41→0.09〜0.12 s、fiber 0.45→0.11〜0.15 s、bppc4-08 1.7→0.56〜0.79 s) が、最初の解の値は悪い (dcmulti 194485→211472、fiber 614432→1255419、bppc4-08 94→108)。総時間・最終の目的値への影響は揺れの範囲 (fiber は mix で 2 組とも少し短い 1.35/1.18 vs 1.69/1.61)。
- mzzv42z: 時間切れはどの設定も同じ。最初の解は mix・mixrole で値が良い (-990→-3620) が、60 秒時点の目的値は newdef -20220/-20470、mix -20240/-20230、mixrole -20170/-19442 と揺れが大きく、向きは揃わない。mix HLa と mixrole HLb は 62 秒で、上限を約 2 秒超えて止まった (ノードが 74 / 47 進んでいる回)。
- gr4x6・nw04: 差なし。
