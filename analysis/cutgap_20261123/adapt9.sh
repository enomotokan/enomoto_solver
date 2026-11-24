#!/bin/bash
# 9 節の確かめ: 9 問 × (既定・試作・適応形) × (1・4 スレッド)、種 1・60 秒、1 本ずつ順に。→ adapt9.jsonl (名札 def9・enum9・adapt9)
cd "$(dirname "$0")"
export ADAPT_OUT=adapt9.jsonl
E="ENOMOTO_MIP_ENUM_PRESOLVE=all ENOMOTO_MIP_ENUM_WHERE=loop ENOMOTO_MIP_PRR_TIGHT=1"
for p in net12 istanbul-no-cutoff ns1830653 noswot mzzv42z neos8 nu25-pr12 air03 neos-960392; do
  for t in 1 4; do
    python adapt_run.py $p $t def9
    python adapt_run.py $p $t enum9 $E
    python adapt_run.py $p $t adapt9 ENOMOTO_MIP_ENUM_ADAPT=1
  done
done
