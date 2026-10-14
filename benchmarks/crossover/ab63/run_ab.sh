#!/bin/bash
# クロスオーバーの双対の押し出し (IPX 式) と、主の押し出しの FT 更新の上限・時間切れで諦める (各問題で交互に計測):
#   base: 従来 (双対の押し出しなし、主の押し出しの FT 更新の上限は単体法と同じ、諦めなし、双対化の列の非零数の上限 45)
#   new:  既定
# nk.jsonl は双対の押し出しを頂点の判定の前に置いた版、nk2.jsonl は仕上げの直前に移した版 (どちらも諦めの規則の前)、
# nk3.jsonl は origin/develop を取り込んだ後 (特異のときの取り消しあり、双対の押し出しは頂点が主実行不能でも行う)、
# nk4.jsonl は最終版 (取り消しなし、双対の押し出しは主実行可能な頂点のときだけ)。
cd "$(dirname "$0")/../../.."
V="base=ENOMOTO_T_XO_DUAL_MEGIDDO=0,ENOMOTO_T_XO_MEGIDDO_FT_MAX=1000000000,ENOMOTO_T_XO_MEGIDDO_GIVEUP=0,ENOMOTO_T_XO_DUALIZE_MAX_COL_NNZ=45 new="
.venv/bin/python scripts/crossover_bench/run.py --set netlib kennington --variants $V \
  --reps 3 --single-run-above 60 --time-limit 600 --out benchmarks/crossover/ab63/nk4.jsonl >> benchmarks/crossover/ab63/nk4.log 2>&1
echo ALL_DONE >> benchmarks/crossover/ab63/done.txt
