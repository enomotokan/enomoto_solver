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

## 追加: 直した版 e88bf56 (mix2 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`)

順番: mix2 HLc → newdef HLc → newdef HLd → mix2 HLd。目的値の不一致・異常終了はなし。

| 問題 | 回 | 状態 | 目的値 | 時間 | 最初の解 | ノード |
|---|---|---|---:|---:|---|---:|
| gr4x6 | newdef HLc / HLd | optimal | 202.35 | 0.06 / 0.04 | 0.032 / 0.033 (229.05) | 0 |
| | mix2 HLc / HLd | optimal | 202.35 | 0.06 / 0.04 | 0.034 / 0.013 (229.05) | 0 |
| dcmulti | newdef HLc / HLd | optimal | 188182 | 1.06 / 1.06 | 0.456 / 0.423 (194485.1) | 4 |
| | mix2 HLc / HLd | optimal | 188182 | 1.10 / 1.24 | 0.115 / 0.116 (211471.7) | 4 |
| fiber | newdef HLc / HLd | optimal | 405935.18 | 1.39 / 1.44 | 0.449 / 0.501 (614431.83) | 3 / 1 |
| | mix2 HLc / HLd | optimal | 405935.18 | 1.44 / 1.51 | 0.127 / 0.135 (1255418.51) | 1 / 1 |
| nw04 | newdef HLc / HLd | optimal | 16862 | 7.82 / 7.23 | 7.631 / 7.119 (52354) | 0 |
| | mix2 HLc / HLd | optimal | 16862 | 8.21 / 8.08 | 8.033 / 7.933 (53592) | 0 |
| bppc4-08 | newdef HLc / HLd | time_limit | 54 / 54 | 60.06 / 60.05 | 1.617 / 1.728 (94) | 29616 / 22534 |
| | mix2 HLc / HLd | time_limit | 54 / 54 | 60.07 / 60.06 | 0.664 / 0.638 (108) | 29347 / 21424 |
| neos-950242 | newdef HLc / HLd | optimal | 4 | 15.41 / 16.04 | 8.425 / 8.831 (19) | 0 |
| | mix2 HLc / HLd | optimal | 4 | 8.65 / 8.96 | 8.476 / 8.848 (4) | 0 |
| mzzv42z | newdef HLc / HLd | time_limit | -20180 / -20370 | 60.07 / 60.04 | 10.658 / 9.272 (-990) | 72 / 0 |
| | mix2 HLc / HLd | time_limit | -20400 / -19780 | 60.04 / 62.05 | 7.107 / 7.241 (-3620) | 0 / 3 |

- 傾向は f970f87 の mix と同じ。neos-950242 は mix2 で 15.4/16.0 s → 8.7/9.0 s (最初の解が最適 4)。dcmulti・fiber・bppc4-08 は最初の解が早いが値は悪く、総時間は揺れの範囲 (dcmulti はやや長い 1.10/1.24 vs 1.06/1.06)。
- nw04 は mix2 が 2 組とも少し長い (8.21/8.08 vs 7.82/7.23、最初の解が 0.4〜0.8 s 遅い)。f970f87 の 6 回では揃っていなかったので、揺れの可能性あり。
- mzzv42z は向きが揃わない。mix2 HLd がまた 62 秒で止まった (上限を約 2 秒超過)。

## 追加 3: 最後の版 95a5f4d (mix3 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、newdef3 = 既定)

順番: mix3 HLe → newdef3 HLe → newdef3 HLf → mix3 HLf。目的値の不一致・異常終了はなし。cand_logs に `WARNING` は 0 件 (結果の .md には警告の列がない)。

| 問題 | 回 | 状態 | 目的値 | 時間 | 最初の解 | ノード |
|---|---|---|---:|---:|---|---:|
| gr4x6 | newdef3 HLe / HLf | optimal | 202.35 | 0.05 / 0.04 | 0.016 / 0.014 (229.05) | 0 |
| | mix3 HLe / HLf | optimal | 202.35 | 0.04 / 0.02 | 0.036 / 0.007 (229.05) | 0 |
| dcmulti | newdef3 HLe / HLf | optimal | 188182 | 1.16 / 1.02 | 0.402 / 0.460 (194485.1) | 4 |
| | mix3 HLe / HLf | optimal | 188182 | 1.14 / 1.20 | 0.117 / 0.118 (211471.7) | 4 |
| fiber | newdef3 HLe / HLf | optimal | 405935.18 | 1.64 / 1.34 | 0.407 (614431.83) / 0.457 (775178.80) | 3 / 1 |
| | mix3 HLe / HLf | optimal | 405935.18 | 1.51 / 1.37 | 0.135 / 0.137 (1255418.51) | 1 / 1 |
| nw04 | newdef3 HLe / HLf | optimal | 16862 | 7.60 / 7.67 | 7.470 / 7.520 (52354) | 0 |
| | mix3 HLe / HLf | optimal | 16862 | 8.56 / 7.98 | 7.911 / 7.239 (53592) | 0 |
| bppc4-08 | newdef3 HLe / HLf | time_limit | 54 / 54 | 60.04 / 60.05 | 1.693 / 1.543 (94) | 23873 / 26267 |
| | mix3 HLe / HLf | time_limit | 55 / 54 | 60.09 / 60.04 | 0.920 (55) / 0.732 (108) | 30208 / 29421 |
| neos-950242 | newdef3 HLe / HLf | optimal | 4 | 15.39 / 12.90 | 8.524 / 8.157 (19) | 0 / 1 |
| | mix3 HLe / HLf | **time_limit・解なし** | - | 60.20 / 60.25 | なし | 0 |
| mzzv42z | newdef3 HLe / HLf | time_limit | -20430 / -19800 | 60.05 / 62.05 | 9.991 / 8.965 (-990) | 0 / 6 |
| | mix3 HLe / HLf | time_limit | -20300 / -20040 | 60.05 / 60.07 | 7.342 / 8.155 (-3620) | 0 |

- **neos-950242 の後退**: mix3 は 2 回とも解なしで時間切れ (e88bf56 の mix2 では最初の解が最適 4 で 8.7/9.0 s)。ログを取って 1 回再現した (`analysis/heur_ls_20261023/logs/neos-950242_mix3s4_1.log`)。`MIP: Local-MIP solution accepted false (5.92s)` で Local-MIP の解が受け入れられず、FJ の解も出ないので、find thread が回らない。根のヒューリスティクスは RENS が 2 本とも Infeasible、feasibility pump が 46 s かかり、57.4 s まで続く。最後に `symmetry: 0 heuristic solutions mapped ..., 2 could not be` が出る。`WARNING` は出ていない。同じ機械の既定 (`logs/neos-950242_def3s4_1.log`) は FJ で 19 (6.3 s)、find thread で 4 (8.8 s) が出て、16.1 s で最適になる。
- 時間の上限の超過: mix3 は 4 回とも 60.1 s 以内に収まった (直っている)。ただし newdef3 HLf の mzzv42z は 62.05 s (既定でも 2 秒超えることがある)。
- 他は f970f87・e88bf56 と同じ傾向 (dcmulti・fiber・bppc4-08 は mix3 で最初の解が早く、値は悪い。nw04 は mix3 が 2 組とも少し長い 8.56/7.98 vs 7.60/7.67)。

## 追加 4: 8352106 (mix4 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、newdef4 = 既定)

順番: mix4 HLg → newdef4 HLg → newdef4 HLh → mix4 HLh。最適と報告した回の目的値の不一致はなし。`WARNING` は cand_logs で 0 件 (JSON に mip_warnings の項目はない)。

| 問題 | 回 | 状態 | 目的値 | 時間 | 最初の解 | ノード |
|---|---|---|---:|---:|---|---:|
| gr4x6 | newdef4 HLg / HLh | optimal | 202.35 | 0.05 / 0.02 | 0.018 / 0.009 (229.05) | 0 |
| | mix4 HLg / HLh | optimal | 202.35 | 0.04 / 0.06 | 0.023 / 0.029 (229.05) | 0 |
| dcmulti | newdef4 HLg / HLh | optimal | 188182 | 1.07 / 1.07 | 0.436 / 0.447 (194485.1) | 4 |
| | mix4 HLg / HLh | optimal | 188182 | 1.06 / 1.22 | 0.112 / 0.154 (211471.7) | 4 |
| fiber | newdef4 HLg / HLh | optimal | 405935.18 | 1.59 / 1.83 | 0.484 / 0.519 (775178.8) | 1 / 3 |
| | mix4 HLg / HLh | optimal | 405935.18 | 1.52 / 1.49 | 0.142 / 0.162 (1255418.5) | 3 / 1 |
| nw04 | newdef4 HLg / HLh | optimal | 16862 | 7.68 / 7.92 | 7.497 / 7.772 (52354) | 0 |
| | mix4 HLg / HLh | optimal | 16862 | 8.20 / 9.74 | 7.453 / 8.949 (53592) | 0 |
| bppc4-08 | newdef4 HLg / HLh | time_limit | 54 / 54 | 60.05 / 60.05 | 1.564 / 1.789 (94) | 28866 / 26547 |
| | mix4 HLg / HLh | time_limit | 55 / 55 | 60.31 / 60.08 | 0.634 / 0.603 (108) | 26876 / 29991 |
| neos-950242 | newdef4 HLg / HLh | optimal | 4 | 13.33 / 15.54 | 8.163 / 8.511 (19) | 1 / 0 |
| | mix4 HLg / HLh | optimal / **time_limit** | 4 / **7** | 15.62 / **62.12** | 8.784 / 8.673 (24) | 0 / 74 |
| mzzv42z | newdef4 HLg / HLh | time_limit | -20270 / -19832 | 60.04 / 62.06 | 8.658 / 8.789 (-990) | 0 / 2 |
| | mix4 HLg / HLh | time_limit | -19832 / -20010 | 62.05 / 60.12 | 6.492 / 7.617 (0) | 2 / 0 |

- **neos-950242**: 解なしは直ったが、mix4 HLh は 7 で時間切れになり、上限を 2 秒超えて 62.12 s で止まった。HLg も 15.62 s で、mix2 (8.7/9.0 s) より遅く newdef4 (13.3/15.5 s) と同じくらい。ログを取って 2 回再現した (`logs/neos-950242_mix4s4_{1,2}.log`、2 回とも時間切れ・62 s、目的値 7 / 5)。
  - 5.49 s に Local-MIP の解が 24 → 21 → 9 → 8 (または 5) と受け入れられ、最後の 1 つ (最適 4 と思われる) は `accepted false` で拒まれる。
  - incumbent が質の悪い FJ の解ではなく find thread が回らないので、4 が見つからない。根のサブ MIP (RENS・locks・clique・RINS) はすべて Infeasible。
  - ログの最後に `symmetry: 0 heuristic solutions mapped ..., 2 could not be` が出る。
- bppc4-08: mix4 は 2 組とも 55 (newdef4 は 54)。mix3 HLe も 55 だった。
- 上限の超過: mzzv42z は mix4 HLg と newdef4 HLh で 62 s (既定でも起きる)。
- nw04 は mix4 が 2 組とも長い (8.20/9.74 vs 7.68/7.92)。他は前と同じ傾向。
