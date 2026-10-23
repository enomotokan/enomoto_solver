# Local-MIP 試作の計測 s5 (2026-10-23)

- ソルバー: f970f87 (台本は inv/heur-ls-20261023 の 1701b36)、4 スレッド・60 秒、`bench_mix.sh`。
- 設定: `newdef` = 既定、`mix` = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、`mixrole` = `ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_ROLE_LMIP=1`。
- 順番: newdef HLa → mix HLa → mixrole HLa → mixrole HLb → mix HLb → newdef HLb。
- 担当: markshare_4_0 gt2 noswot assign1-5-8 cvs16r128-89 drayage-25-23 neos-960392。

## まとめ

- 目的値の不一致 (optimal と言って最適値と違う) や異常終了はなし。
- 2 組とも同じ向きの差:
  - cvs16r128-89 (最小化、60 秒で打ち切り): mix・mixrole で最終解が -87〜-93 (newdef は 2 回とも -51)。最初の解も 1.6〜2.0 秒で -79 (newdef は 4 秒で -5)。良くなる。
  - neos-960392: newdef は -237 (時間切れ) / -238 (28.6 秒で最適)。mix は 2 回とも -230、mixrole は 2 回とも -213 で時間切れ。最初の解は少し早い (12.5 秒 vs 15 秒) が、節点がほとんど進まず悪くなる。
  - assign1-5-8: 最終値は 212 で同じだが、最初の解が 0.3〜0.6 秒で 214 (newdef は 1.5 秒で 248)。
  - drayage-25-23: 全部 23〜31 秒で最適。最初の解が mix では 14 秒・985825 と遅く悪い (newdef 5 秒・225984)、mixrole では 2.3〜2.6 秒・293676 と早い。
  - markshare_4_0: 全部最適だが mix・mixrole で 2.6〜3.9 秒 (newdef 1.9〜2.5 秒)。差は小さい。
- noswot は mixrole HLb だけ時間切れ (-41、下界 -43)。他の回は 27〜49 秒で最適。1 回だけなので揺れの範囲と見る。
- gt2 は全部 0.1 秒で最適 (最初の解の値は mix 系が悪いが影響なし)。

## 問題ごとの結果

### markshare_4_0 (HiGHS: Time limit reached 1 / 60.0s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 1 | 1 | 1.9 | 203257 | 0.03 | 175 |
| mix | HLa | optimal | 1 | 1 | 2.6 | 236932 | 0.01 | 175 |
| mixrole | HLa | optimal | 1 | 1 | 2.8 | 263369 | 0.00 | 175 |
| mixrole | HLb | optimal | 1 | 1 | 3.1 | 292178 | 0.01 | 175 |
| mix | HLb | optimal | 1 | 1 | 3.9 | 297316 | 0.01 | 175 |
| newdef | HLb | optimal | 1 | 1 | 2.5 | 214469 | 0.01 | 175 |

### gt2 (HiGHS: Optimal 21166 / 0.0s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 21166 | 21166 | 0.1 | 0 | 0.07 | 52776 |
| mix | HLa | optimal | 21166 | 21166 | 0.1 | 0 | 0.05 | 187545 |
| mixrole | HLa | optimal | 21166 | 21166 | 0.1 | 0 | 0.01 | 187545 |
| mixrole | HLb | optimal | 21166 | 21166 | 0.1 | 0 | 0.02 | 187545 |
| mix | HLb | optimal | 21166 | 21166 | 0.1 | 0 | 0.01 | 187545 |
| newdef | HLb | optimal | 21166 | 21166 | 0.1 | 0 | 0.06 | 52776 |

### noswot (HiGHS: Time limit reached -41 / 60.0s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | -41 | -41 | 33.6 | 57439 | 0.07 | -14 |
| mix | HLa | optimal | -41 | -41 | 27.0 | 44555 | 0.03 | -14 |
| mixrole | HLa | optimal | -41 | -41 | 38.5 | 57863 | 0.04 | -14 |
| mixrole | HLb | time_limit | -41 | -43 | 60.0 | 85739 | 0.05 | -14 |
| mix | HLb | optimal | -41 | -41 | 43.4 | 58769 | 0.04 | -14 |
| newdef | HLb | optimal | -41 | -41 | 48.8 | 70831 | 0.08 | -14 |

### assign1-5-8 (HiGHS: Time limit reached 212 / 60.0s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | 212 | 193.916 | 60.1 | 136226 | 1.44 | 248 |
| mix | HLa | time_limit | 212 | 194.642 | 60.1 | 131341 | 0.30 | 214 |
| mixrole | HLa | time_limit | 212 | 193.607 | 60.1 | 132498 | 0.39 | 214 |
| mixrole | HLb | time_limit | 212 | 193.228 | 60.0 | 105651 | 0.59 | 214 |
| mix | HLb | time_limit | 212 | 193.647 | 60.0 | 124176 | 0.48 | 214 |
| newdef | HLb | time_limit | 212 | 194.335 | 60.1 | 141156 | 1.52 | 248 |

### cvs16r128-89 (HiGHS: Time limit reached -3 / 60.0s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | -51 | -126.318 | 62.0 | 9 | 4.20 | -5 |
| mix | HLa | time_limit | -93 | -126.628 | 62.1 | 794 | 1.59 | -79 |
| mixrole | HLa | time_limit | -89 | -126.628 | 61.8 | 772 | 1.66 | -79 |
| mixrole | HLb | time_limit | -87 | -126.656 | 60.5 | 424 | 1.65 | -79 |
| mix | HLb | time_limit | -92 | -126.656 | 62.0 | 558 | 1.97 | -79 |
| newdef | HLb | time_limit | -51 | -126.318 | 62.1 | 15 | 4.02 | -5 |

### drayage-25-23 (HiGHS: Optimal 101283 / 1.6s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | optimal | 101283 | 101283 | 25.6 | 0 | 5.27 | 225984 |
| mix | HLa | optimal | 101283 | 101283 | 23.7 | 0 | 14.02 | 985825 |
| mixrole | HLa | optimal | 101283 | 101283 | 23.1 | 0 | 2.26 | 293676 |
| mixrole | HLb | optimal | 101283 | 101283 | 30.9 | 17 | 2.56 | 293676 |
| mix | HLb | optimal | 101283 | 101283 | 28.7 | 4747 | 14.72 | 985825 |
| newdef | HLb | optimal | 101283 | 101283 | 23.3 | 0 | 5.16 | 225984 |

### neos-960392 (HiGHS: Optimal -238 / 13.9s)

| 設定 | 回 | 状態 | 目的値 | 下界 | 時間 (s) | 節点 | 最初の解 (s) | 最初の解の値 |
|---|---|---|---|---|---|---|---|---|
| newdef | HLa | time_limit | -237 | -238 | 60.5 | 1 | 14.97 | -2 |
| mix | HLa | time_limit | -230 | -238 | 62.1 | 125 | 12.70 | -35 |
| mixrole | HLa | time_limit | -213 | -238 | 61.6 | 18 | 12.55 | -35 |
| mixrole | HLb | time_limit | -213 | -238 | 62.1 | 4 | 12.76 | -35 |
| mix | HLb | time_limit | -230 | -238 | 62.1 | 92 | 12.44 | -35 |
| newdef | HLb | optimal | -238 | -238 | 28.6 | 0 | 15.26 | -2 |
