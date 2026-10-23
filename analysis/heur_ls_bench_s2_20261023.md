# Local-MIP 試作の計測 s2 (2026-10-23)

- 対象: inv/heur-ls-20261023 の 1701b36 (ソルバーは f970f87 と同じ)、4 スレッド・60 秒、`analysis/heur_ls_20261023/bench_mix.sh`
- 設定: `newdef` = 既定、`mix` = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、`mixrole` = `ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_ROLE_LMIP=1`
- 担当: gen-ip054 misc07 air03 neos-860300 supportcase26 hypothyroid-k1 uccase12
- 順番: newdef HLa → mix HLa → mixrole HLa → mixrole HLb → mix HLb → newdef HLb
- 結果: `benchmarks/{miplib_results,highsonly16}_4t_60s_20261020_f970f87_<設定>_s2_rHL{a,b}.json/.md`、`benchmarks/cand_logs/<設定>_s2_rHL*.log`

## 目立った差 (2 組とも同じ向き)

- **hypothyroid-k1**: newdef は 2 回とも最適 (-2851、36〜38 秒) だが、mix・mixrole は 4 回とも時間切れで、目的値は最初の解の -3 のまま (下界は -2887 前後)。最初の解から先に進んでいない。**大きな後退。**
- 最初の解は mix・mixrole が早い (misc07 0.75→0.13 秒、neos-860300 7.1→4.6 秒、supportcase26 1.05→0.30 秒、gen-ip054 0.05〜0.09→0.02〜0.04 秒)。ただし値は misc07・neos-860300・supportcase26 では newdef より悪く、gen-ip054 だけ良い (20166→11961)。
- 最終の目的値・時間の差は揺れの幅に収まる (supportcase26 は mix が 2 回とも newdef より良い 1770/1756 対 1790/1785 が、mixrole はそうならない。neos-860300 は mix・mixrole がやや遅い傾向 49〜54 秒対 44〜49 秒)。

## 目的値の不一致・異常終了

- 異常終了はない。最適と答えた回の目的値はすべて HiGHS と一致 (misc07 2810、air03 340160、neos-860300 3201、hypothyroid-k1 -2851)。
- air03 の mix HLa は最初の解の値が 675502 (ほかは 340160)。最終値は同じ。

## 問題ごとの結果

時間・時刻の単位は秒。

### gen-ip054

HiGHS(newdef HLa): Time limit reached 目的値 6860.19 時間 60.0091

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | 6847.25 | 6790.7 | 60.0705 | 0.0851479 | 20165.7 |
| mix | HLa | time_limit | 6840.97 | 6786.82 | 60.0825 | 0.0237036 | 11961.1 |
| mixrole | HLa | time_limit | 6855.71 | 6788.95 | 60.0967 | 0.0398659 | 11961.1 |
| mixrole | HLb | time_limit | 6853.16 | 6787.51 | 60.077 | 0.0150306 | 11961.1 |
| mix | HLb | time_limit | 6840.97 | 6788.85 | 60.0598 | 0.0169782 | 11961.1 |
| newdef | HLb | time_limit | 6840.97 | 6788.11 | 60.0507 | 0.0546232 | 20165.7 |

### misc07

HiGHS(newdef HLa): Optimal 目的値 2810 時間 27.7623

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 2810 | 2810 | 21.1527 | 0.790324 | 3690 |
| mix | HLa | optimal | 2810 | 2810 | 18.3336 | 0.134482 | 4225 |
| mixrole | HLa | optimal | 2810 | 2810 | 23.9733 | 0.160021 | 4225 |
| mixrole | HLb | optimal | 2810 | 2810 | 21.6793 | 0.122562 | 4225 |
| mix | HLb | optimal | 2810 | 2810 | 19.2013 | 0.143981 | 4225 |
| newdef | HLb | optimal | 2810 | 2810 | 19.3756 | 0.755997 | 3690 |

### air03

HiGHS(newdef HLa): Optimal 目的値 340160 時間 2.20065

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 340160 | 340160 | 3.97156 | 3.95563 | 340160 |
| mix | HLa | optimal | 340160 | 340160 | 3.53568 | 3.51846 | 675502 |
| mixrole | HLa | optimal | 340160 | 340160 | 3.94747 | 3.93116 | 340160 |
| mixrole | HLb | optimal | 340160 | 340160 | 3.79139 | 3.77108 | 340160 |
| mix | HLb | optimal | 340160 | 340160 | 3.52427 | 3.5018 | 340160 |
| newdef | HLb | optimal | 340160 | 340160 | 3.56808 | 3.54007 | 340160 |

### neos-860300

HiGHS(newdef HLa): Optimal 目的値 3201 時間 14.3945

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 3201 | 3201 | 48.9488 | 7.17031 | 5497 |
| mix | HLa | optimal | 3201 | 3201 | 53.6647 | 4.62592 | 10252 |
| mixrole | HLa | optimal | 3201 | 3201 | 49.0118 | 4.6219 | 10252 |
| mixrole | HLb | optimal | 3201 | 3201 | 53.3509 | 4.62118 | 10252 |
| mix | HLb | optimal | 3201 | 3201 | 50.4561 | 4.92738 | 10252 |
| newdef | HLb | optimal | 3201 | 3201 | 44.2836 | 7.03451 | 5497 |

### supportcase26

HiGHS(newdef HLa): Time limit reached 目的値 1790.05 時間 60.007

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | 1789.5 | 1436.99 | 60.0319 | 1.06433 | 1916.88 |
| mix | HLa | time_limit | 1769.83 | 1433.29 | 60.0618 | 0.341482 | 2201.88 |
| mixrole | HLa | time_limit | 1797.94 | 1434.76 | 60.0571 | 0.307945 | 2201.88 |
| mixrole | HLb | time_limit | 1781 | 1432.63 | 60.0494 | 0.326354 | 2201.88 |
| mix | HLb | time_limit | 1755.53 | 1437.14 | 60.035 | 0.29531 | 2201.88 |
| newdef | HLb | time_limit | 1785.25 | 1435.94 | 60.0481 | 1.04288 | 1916.88 |

### hypothyroid-k1

HiGHS(newdef HLa): Optimal 目的値 -2851 時間 12.8183

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | -2851 | -2851 | 37.5145 | 15.0677 | -3 |
| mix | HLa | time_limit | -3 | -2887.82 | 60.2372 | 15.6426 | -3 |
| mixrole | HLa | time_limit | -3 | -2888.36 | 60.3738 | 14.6312 | -3 |
| mixrole | HLb | time_limit | -3 | -2887.82 | 60.0841 | 14.5733 | -3 |
| mix | HLb | time_limit | -3 | -2887 | 60.7036 | 13.0381 | -3 |
| newdef | HLb | optimal | -2851 | -2851 | 36.3175 | 16.4715 | -3 |

### uccase12

HiGHS(newdef HLa): Time limit reached 目的値 11507.4 時間 60.0546

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | 11507.9 | 11507.3 | 61.094 | 45.9241 | 902357 |
| mix | HLa | time_limit | 11622.8 | 11507.4 | 62.8995 | 33.6347 | 852383 |
| mixrole | HLa | time_limit | 11507.9 | 11507.3 | 60.8201 | 36.1196 | 836516 |
| mixrole | HLb | time_limit | 11507.9 | 11507.4 | 60.6508 | 35.8825 | 824563 |
| mix | HLb | time_limit | 11507.9 | 11507.4 | 60.8036 | 33.5338 | 835145 |
| newdef | HLb | time_limit | 11622.8 | 11507.4 | 61.0399 | 42.8462 | 902357 |
