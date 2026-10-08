#!/bin/bash
# fome13 の当たり外れの対策 (各問題で交互に計測):
#   default: 既定
#   rrs: 分解の破綻のときに近接中心を今の点に置き直す (ENOMOTO_T_IPM_BREAK_RECENTER=1) + 主実行不能が少ない頂点を
#        費用をずらした双対単体法で直す (ENOMOTO_T_XO_REPAIR=1) + 高精度化の停滞を 2 回まで許す (ENOMOTO_T_IPM_PUSH_STALL=2)
#   mf_rrs: rrs + マルチフロンタル法 (ENOMOTO_T_CHOL_BACKEND=2)
cd "$(dirname "$0")/../../.."
R=ENOMOTO_T_IPM_BREAK_RECENTER=1,ENOMOTO_T_XO_REPAIR=1,ENOMOTO_T_IPM_PUSH_STALL=2
V="default= rrs=$R mf_rrs=$R,ENOMOTO_T_CHOL_BACKEND=2"
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab60/nk.jsonl >> benchmarks/crossover/ab60/nk.log 2>&1
P="qap15 s250r10 scpm1 fome13 ns1688926 ex10"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 300 --out benchmarks/crossover/ab60/mitt.jsonl >> benchmarks/crossover/ab60/mitt.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab60/done.txt
