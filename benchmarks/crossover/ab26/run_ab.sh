#!/bin/bash
# 非基底の検出の質の検証 (ENOMOTO_T_XO_DETECT_EVAL=1): 同じ内点法の点で γ = 1・γ = τ_j・PDHG の 1 歩 + γ = τ_j の分類を作り、
# 仕上げの後の最適基底と突き合わせる。最適基底が検出の方式に引きずられる偏りを見るため、既定 (PDHG の 1 歩) と γ = 1 の
# 両方で解く。時間は見ないので 1 回ずつ。
cd "$(dirname "$0")/../../.."
until [ -e benchmarks/crossover/ab25/done.txt ]; do sleep 30; done
. .venv/bin/activate && maturin develop --release > benchmarks/crossover/ab26/build.log 2>&1
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington \
  --variants pdhg=ENOMOTO_T_XO_DETECT_EVAL=1 gamma1=ENOMOTO_T_XO_DETECT_EVAL=1,ENOMOTO_T_XO_GAMMA_PDHG=0 \
  --reps 1 --time-limit 600 --out benchmarks/crossover/ab26/eval.jsonl >> benchmarks/crossover/ab26/eval.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab26/done.txt
