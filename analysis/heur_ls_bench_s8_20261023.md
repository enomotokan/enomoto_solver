# Local-MIP 試作 計測 s8 (2026-10-23)

対象: f970f87 (inv/heur-ls-20261023 の 1701b36 の台本で、問題を絞って計測)、4 スレッド・60 秒。
設定: newdef = 既定、mix = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、mixrole = mix + `ENOMOTO_MIP_ROLE_LMIP=1`。
順番: newdef HLa → mix HLa → mixrole HLa → mixrole HLb → mix HLb → newdef HLb。
列: 状態 / 目的値 / 限界値 / 時間 (秒) / 最初の解の時刻 (秒) / 最初の解の値。

## neos5 (HiGHS: time limit, 15)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | time_limit | 15 | 14.301 | 60.04 | 0.036 | 17 |
| mix HLa | time_limit | 15 | 14.333 | 60.02 | 0.048 | 17 |
| mixrole HLa | time_limit | 15 | 14.321 | 60.02 | 0.030 | 17 |
| mixrole HLb | time_limit | 15 | 14.333 | 60.02 | 0.035 | 17 |
| mix HLb | time_limit | 15 | 14.313 | 60.02 | 0.035 | 17 |
| newdef HLb | time_limit | 15 | 14.341 | 60.02 | 0.028 | 17 |

## rout (HiGHS: optimal 1077.56, 30.13 秒)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | optimal | 1077.56 | 1077.56 | 12.18 | 0.813 | 2348.69 |
| mix HLa | optimal | 1077.56 | 1077.56 | 13.83 | 0.141 | 2375.25 |
| mixrole HLa | optimal | 1077.56 | 1077.56 | 19.17 | 0.133 | 2375.25 |
| mixrole HLb | optimal | 1077.56 | 1077.56 | 10.76 | 0.109 | 2375.25 |
| mix HLb | optimal | 1077.56 | 1077.56 | 17.61 | 0.108 | 2375.25 |
| newdef HLb | optimal | 1077.56 | 1077.56 | 13.49 | 0.565 | 2348.69 |

## 30n20b8 (HiGHS: time limit, 553)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | time_limit | 653 | 140.58 | 60.13 | 59.031 | 653 |
| mix HLa | time_limit | 302 | 139.76 | 60.21 | 4.997 | 453 |
| mixrole HLa | time_limit | 302 | 139.77 | 60.33 | 4.903 | 453 |
| mixrole HLb | time_limit | 353 | 140.20 | 60.47 | 5.135 | 806 |
| mix HLb | time_limit | 353 | 139.59 | 60.16 | 5.366 | 453 |
| newdef HLb | time_limit | **解なし** | 139.56 | 60.71 | — | — |

## ran14x18-disj-8 (HiGHS: time limit, 3736)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | time_limit | 3748 | 3564.97 | 60.04 | 0.240 | 12092 |
| mix HLa | time_limit | 3736 | 3556.39 | 60.03 | 0.230 | 12092 |
| mixrole HLa | time_limit | 3736 | 3552.62 | 60.04 | 0.281 | 12092 |
| mixrole HLb | time_limit | 3736 | 3564.41 | 60.04 | 0.264 | 12092 |
| mix HLb | time_limit | 3736 | 3561.03 | 60.03 | 0.244 | 12092 |
| newdef HLb | time_limit | 3751 | 3555.27 | 60.04 | 0.228 | 12092 |

## graph20-20-1rand (HiGHS: time limit, -8)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | time_limit | -8 | -36.00 | 62.05 | 4.159 | -3 |
| mix HLa | time_limit | -9 | -36.00 | 60.44 | 1.562 | -8 |
| mixrole HLa | time_limit | -9 | -35.59 | 62.05 | 1.604 | -8 |
| mixrole HLb | time_limit | -9 | -35.59 | 60.55 | 1.563 | -8 |
| mix HLb | time_limit | -9 | -35.30 | 60.35 | 1.566 | -8 |
| newdef HLb | time_limit | -8 | -36.00 | 60.52 | 4.057 | -3 |

## mc11 (HiGHS: optimal 11689, 55.13 秒)
| 回 | 状態 | 目的値 | 限界値 | 時間 | 最初の解 時刻 | 最初の解 値 |
|---|---|---|---|---|---|---|
| newdef HLa | optimal | 11689 | 11689 | 45.06 | 0.252 | 13548 |
| mix HLa | optimal | 11689 | 11689 | 27.15 | 0.276 | 13548 |
| mixrole HLa | optimal | 11689 | 11689 | 53.75 | 0.282 | 13548 |
| mixrole HLb | optimal | 11689 | 11689 | 56.21 | 0.307 | 13548 |
| mix HLb | optimal | 11689 | 11689 | 42.61 | 0.296 | 13548 |
| newdef HLb | optimal | 11689 | 11689 | 44.98 | 0.300 | 13548 |

## まとめ
- 目的値の不一致・異常終了なし (optimal はすべて HiGHS と一致)。newdef HLb の 30n20b8 は 60 秒で解なし (異常終了ではない)。
- 2 組とも同じ向きの差:
  - 30n20b8: mix・mixrole は約 5 秒で最初の解 (newdef は 59 秒 / 解なし)、最終値 302〜353 で HiGHS (553) より良い。
  - graph20-20-1rand: mix・mixrole は最初の解が 1.6 秒・-8 (newdef 4.1 秒・-3)、最終値 -9 (newdef・HiGHS は -8)。
  - ran14x18-disj-8: mix・mixrole は 3736 (newdef 3748/3751)。rout: mix・mixrole は最初の解が早い (0.11〜0.14 秒、newdef 0.57〜0.81 秒)。
- mixrole は mc11 の時間が 2 回とも newdef より遅い (53.8/56.2 秒、newdef 45 秒)。mix は 27/43 秒で揺れが大きい。
