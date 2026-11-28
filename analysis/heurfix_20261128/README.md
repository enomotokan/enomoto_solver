# HEUR_ADAPT の漏れの直し `ENOMOTO_MIP_HEUR_ADAPT` の `r` (2026-11-28、親の依頼・利用者の承認)

analysis/combo3_20261126 8.3 節: HEUR_ADAPT は根の LP の反復の物差し `Solver::root_lp_iters` を、前処理の LP の最初の LP の反復の側へ f_g だけ寄せる
(根のヒューリスティクスの予算のため)。この値が GAP1 u の LP プロービングの子の反復の上限・DFSBB・LNS・強分枝の単位などにも漏れていた。

## 直し (src/mip/solver.rs、既定オフ)

`ENOMOTO_MIP_HEUR_ADAPT` に文字 `r` を足した (`=1mkr` など)。`r` では寄せた値を根のヒューリスティクスの予算 (局所の `root_iters`) にだけ使い、
`self.root_lp_iters` は寄せない。新しい閾値はない。HEUR_ADAPT がオフ、または `r` がなければ道筋は変わらない (check_default 一致)。

## 確かめ (`res_def.jsonl`・`res_r.jsonl`、種 1、600 秒、先端 9b56cb34d + 直し)

| 問題 | スレッド | 既定 | c2c (HEUR_ADAPT=1mk) | c2c + `r` (1mkr) |
|---|---|---:|---:|---:|
| dano3_3 | 1 | 64.9 | 95.3 | **63.9** |
| piperout-27 | 1 | 16.0 | 24.6 | 26.8 |
| piperout-27 | 4 | 16.4 | 21.6 | 21.7 |
| neos-1582420 | 4 | 22.5 | 23.5 | 32.0 |
| app1-1 | 4 | 8.0 | 12.1 | 11.2 |

- dano3_3 の根の遅れは `r` で消える (既定並み)。
- piperout-27 は戻らない: 別の原因 (前回の切り分けの ENUM_ADAPT と HEUR_ADAPT の根のヒューリスティクスの手間)。
- neos-1582420・app1-1 は 4 スレッドの揺れの幅で、この版では c2c と既定の差が小さく、漏れの形は見えない。
