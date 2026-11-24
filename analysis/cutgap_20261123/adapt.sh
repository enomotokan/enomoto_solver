#!/bin/bash
# 8 節の確かめ: 5 問 × (既定・試作・適応形) × (1・4 スレッド)、種 1・60 秒、1 本ずつ順に。→ adapt.jsonl、表は adapttab.py
cd "$(dirname "$0")"
E="ENOMOTO_MIP_ENUM_PRESOLVE=all ENOMOTO_MIP_ENUM_WHERE=loop ENOMOTO_MIP_PRR_TIGHT=1"
for p in net12 istanbul-no-cutoff ns1830653 noswot mzzv42z; do
  for t in 1 4; do
    python adapt_run.py $p $t def
    python adapt_run.py $p $t enum $E
    python adapt_run.py $p $t adapt ENOMOTO_MIP_ENUM_ADAPT=1
  done
done
