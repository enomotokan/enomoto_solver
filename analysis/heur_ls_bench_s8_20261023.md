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

## 追加計測: e88bf56 (mix の修正後)
順番: mix2 HLc → newdef HLc → newdef HLd → mix2 HLd (mix2 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`)。列は上と同じ。

| 問題 | mix2 HLc | newdef HLc | newdef HLd | mix2 HLd |
|---|---|---|---|---|
| neos5 | 15 / 最初 0.057 秒 | 15 / 0.049 | 15 / 0.031 | 15 / 0.035 |
| rout | optimal 10.65 秒 / 最初 0.139 秒 | optimal 11.62 / 0.821 | optimal 10.93 / 0.777 | optimal 10.23 / 0.157 |
| 30n20b8 | 453 / 最初 9.50 秒 (453) | **解なし** | **解なし** | 302 / 最初 5.02 秒 (453) |
| ran14x18-disj-8 | 3744 | 3748 | 3736 | 3744 |
| graph20-20-1rand | -9 / 最初 1.99 秒 (-8) | -8 / 4.35 (-3) | -8 / 4.34 (-3) | -9 / 1.51 (-8) |
| mc11 | optimal 44.86 秒 | optimal 44.25 | optimal 41.33 | optimal 53.04 |

- 目的値の不一致・異常終了はなし。newdef は 30n20b8 で 2 回とも 60 秒で解なし。
- 2 組とも同じ向き: 30n20b8 (mix2 だけが解を得る)、graph20-20-1rand (mix2 は -9、newdef は -8、最初の解も早い)、rout (mix2 は最初の解が 0.14〜0.16 秒、newdef は 0.78〜0.82 秒)。ran14x18-disj-8 の f970f87 での mix の利点 (3736) は今回は見えない (mix2 3744、newdef は 3748/3736)。

## 追加計測 3: 95a5f4d
順番: mix3 HLe → newdef3 HLe → newdef3 HLf → mix3 HLf (mix3 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`)。

| 問題 | mix3 HLe | newdef3 HLe | newdef3 HLf | mix3 HLf |
|---|---|---|---|---|
| neos5 | 15 / 最初 0.041 秒 | 15 / 0.036 | 15 / 0.036 | 15 / 0.033 |
| rout | optimal 13.75 秒 / 最初 0.122 秒 | optimal 11.07 / 0.648 | optimal 12.58 / 0.809 | optimal 8.12 / 0.109 |
| 30n20b8 | 353 / 最初 6.01 秒 (453) | 553 / 55.03 (804) | 553 / 57.13 (553) | 302 / 5.54 (453) |
| ran14x18-disj-8 | 3748 | 3748 | 3736 | 3748 |
| graph20-20-1rand | -9 / 最初 1.88 秒 (-8) | -8 / 4.07 (-3) | -9 / 4.12 (-3) | -9 / 1.73 (-8) |
| mc11 | optimal 42.04 秒 | optimal 38.72 | optimal 56.21 | optimal 44.35 |

- 目的値の不一致・異常終了はなし (optimal はすべて HiGHS と一致)。
- 結果の .md に警告の列はなく (scripts/bench_cand_20261020.py に警告を出す処理がない)、cand_logs にも "not reporting optimality" は出ていない。
- 2 組とも同じ向き: 30n20b8 (mix3 は約 6 秒で最初の解、最終 302〜353。newdef3 は 55〜57 秒で最初の解、最終 553)、rout と graph20-20-1rand の最初の解 (mix3 のほうが早く、値も良い)。

## 追加計測 4: 8352106
順番: mix4 HLg → newdef4 HLg → newdef4 HLh → mix4 HLh (mix4 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`)。

| 問題 | mix4 HLg | newdef4 HLg | newdef4 HLh | mix4 HLh |
|---|---|---|---|---|
| neos5 | 15 / 最初 0.065 秒 | 15 / 0.027 | 15 / 0.027 | 15 / 0.043 |
| rout | optimal 10.21 秒 / 最初 0.110 秒 | optimal 13.18 / 0.668 | optimal 11.05 / 0.809 | optimal 14.54 / 0.141 |
| 30n20b8 | 302 / 最初 5.53 秒 (453) | 553 / 53.38 (806) | **解なし** | 353 / 4.89 (453) |
| ran14x18-disj-8 | 3748 | 3736 | 3748 | 3736 |
| graph20-20-1rand | -9 / 最初 1.60 秒 (-3) | -8 / 4.40 (-3) | -8 / 4.41 (-3) | -9 / 1.60 (-3) |
| mc11 | optimal 55.56 秒 | optimal 51.94 | optimal 48.55 | optimal 52.79 |

- 目的値の不一致・異常終了はなし。MIP_WARNING は 0 回 (JSON に mip_warnings なし、cand_logs にも "MIP: WARNING" なし)。
- 2 組とも同じ向き: 30n20b8 (mix4 は約 5 秒で最初の解、最終 302/353。newdef4 は 553 か解なし)、graph20-20-1rand (mix4 は -9、newdef4 は -8、最初の解 1.6 秒 vs 4.4 秒)、rout の最初の解 (mix4 0.11〜0.14 秒、newdef4 0.67〜0.81 秒)。

## 追加計測 5: 9e34b8e
順番: newdef5 HLi → mix5 HLi → mixoi5 HLi → mixoi5 HLj → mix5 HLj → newdef5 HLj (mix5 = `ENOMOTO_MIP_FJ_LMIP_MIX=1`、mixoi5 = mix5 + `ENOMOTO_MIP_SYM_ORIG_INC=1`)。

| 問題 | newdef5 HLi | mix5 HLi | mixoi5 HLi | mixoi5 HLj | mix5 HLj | newdef5 HLj |
|---|---|---|---|---|---|---|
| neos5 | 15 | 15 | 15 | 15 | 15 | 15 |
| rout | opt 14.03 秒 / 最初 0.714 | opt 9.40 / 0.130 | opt 11.62 / 0.125 | opt 9.73 / 0.124 | opt 9.64 / 0.146 | opt 9.84 / 0.666 |
| 30n20b8 | 653 / 最初 54.8 秒 | 353 / 5.5 | 353 / 4.8 | 302 / 5.1 | 302 / 4.9 | 604 / 57.6 |
| ran14x18-disj-8 | 3762 | 3748 | 3736 | 3748 | 3735 | 3748 |
| graph20-20-1rand | -8 / 最初 3.94 (-3) | -9 / 1.62 (-8) | -9 / 1.56 (-8) | -9 / 1.76 (-8) | -9 / 1.76 (-8) | -8 / 4.02 (-3) |
| mc11 | opt 53.33 秒 | opt 59.86 | opt 51.58 | opt 44.63 | opt 41.02 | **time_limit** 11689 (限界値 11687.77) |

- 目的値の不一致・異常終了はなし。MIP_WARNING は 0 回 (JSON に mip_warnings なし、cand_logs にも "MIP: WARNING" なし)。newdef5 HLj の mc11 は 60 秒で最適の証明が間に合わなかった (値は正しい)。
- 2 組とも同じ向き (mix5 と newdef5): 30n20b8 (mix5 は約 5 秒で最初の解、最終 302/353。newdef5 は 55〜58 秒、最終 604/653)、graph20-20-1rand (-9 vs -8)、rout の最初の解 (0.13〜0.15 秒 vs 0.67〜0.71 秒)。
- mixoi5 と mix5: 2 組とも同じ向きのはっきりした差はなし。
