#!/bin/bash
# 第 60 回の修正 (各問題で交互に計測):
#   default: 既定
#   rrs2: 分解の破綻のときに近接中心を置き直す (ENOMOTO_T_IPM_BREAK_RECENTER=1) + 主実行不能が少ない頂点を直す
#         (ENOMOTO_T_XO_REPAIR=1) + 高精度化の停滞を 2 回まで許す (ENOMOTO_T_IPM_PUSH_STALL=2) + 頂点の主実行可能の
#         許容 1e-8 (ENOMOTO_T_XO_VERTEX_FEAS_TOL=1e-8、pds-20 は違反 1.1e-9 の 1 個で修復に 10 秒かかった)
#   mf_rrs2: rrs2 + マルチフロンタル法 (演算量の見積もりが 2e7 以上で使う。d6cube 1.1e7 は faer)
cd "$(dirname "$0")/../../.."
R=ENOMOTO_T_IPM_BREAK_RECENTER=1,ENOMOTO_T_XO_REPAIR=1,ENOMOTO_T_IPM_PUSH_STALL=2,ENOMOTO_T_XO_VERTEX_FEAS_TOL=1e-8
V="default= rrs2=$R mf_rrs2=$R,ENOMOTO_T_CHOL_BACKEND=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab61/nk.jsonl >> benchmarks/crossover/ab61/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab61/mitt.jsonl >> benchmarks/crossover/ab61/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab61/done.txt
