# 計測 12 の結果 (62ef950、既定 def12 と ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10 (st12)、2026-10-24)

40 問 + 16 問、60 秒・4 スレッド、組 HLs・HLt の 2 回ずつ。各組の子の報告をまとめたもの。ブランチは inv/stall12-bench-20261024-s1〜s8。
順は def(HLs) / st(HLs) / st(HLt) / def(HLt)。**目的値の不一致は 0**。neos-911970 は 40 問の側で数える。

## 最適に解けた数

| | def HLs | st HLs | st HLt | def HLt |
|---|---|---|---|---|
| 最適 (56 問) | 35 | 32 | 34 | 36 |

- 回によって違うもの: h80x6320d (st HLs が TL、値は最適値)、timtab1 (def HLs だけ 57.7 秒)、neos-2978193-inde (st HLs が TL)、
  noswot (def HLt だけ 56.9 秒)、qiu (HLt の 2 回だけ)、neos-911970 (st HLt が TL)。
- 計測 11 より機械が遅い組が多い (組 1・3・5・7 で 40〜58 秒の問題が多く、60 秒の境目で落ちる)。

## 狙いの問題

| 問題 | def (HLs・HLt) | st (HLs・HLt) | HiGHS |
|---|---|---|---|
| neos-960392 | TL −235・−229 | **TL −191・−191** | opt −238 (13.9 秒) |
| physiciansched6-2 | TL 49724・50425 | TL 49724・50425 | opt 49324 |
| mzzv42z | TL −20480・−20390 | TL −20420・−20410 | opt −20540 (50 秒) |

- neos-960392 は調べの子の run_cfg.sh では 4 回とも最適 (35〜37 秒) だったが、bench_mix.sh では 2 回とも悪化。食い違いの原因を調べの子
  (inv/highstree-n960-20261024) に確かめさせている。

## 見立て

- **SUBMIP_CUT_STALL=10 は既定にしない**。最適の数が 2〜3 減り (境目の問題で証明が遅れる)、狙いの neos-960392 も計測では良くならない。
