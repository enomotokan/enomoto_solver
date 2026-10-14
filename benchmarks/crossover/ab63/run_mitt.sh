#!/bin/bash
# 双対化する Mittelmann の問題で、双対 LP の点を元の問題に戻してクロスオーバーするか (back1、従来の既定) と、
# 双対 LP のままクロスオーバー (双対の押し出しつき) と仕上げを行うか (back0)。どちらも双対の押し出しは既定 (あり)。
#   back1: ENOMOTO_T_XO_DUALIZE_BACK=1、双対化の判定は単体法と同じ (構造列の平均非零数 45 以下)
#   back0: ENOMOTO_T_XO_DUALIZE_BACK=0、内点法 + クロスオーバーの双対化は平均非零数 100 以下 (ex10 を含む)
cd "$(dirname "$0")/../../.."
V="back1=ENOMOTO_T_XO_DUALIZE_BACK=1,ENOMOTO_T_XO_DUALIZE_MAX_COL_NNZ=45 back0=ENOMOTO_T_XO_DUALIZE_BACK=0"
P="ex10 supportcase10 neos graph40-40 rmine15 physiciansched3-3"
.venv/bin/python scripts/crossover_bench/run.py --set mittelmann --only $P --variants $V \
  --reps 1 --time-limit 600 --out benchmarks/crossover/ab63/mitt.jsonl >> benchmarks/crossover/ab63/mitt.log 2>&1
echo MITT_DONE >> benchmarks/crossover/ab63/done.txt
