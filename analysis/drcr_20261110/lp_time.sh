#!/bin/bash
# DRCR の LP の時間 (analysis/drcr_20261110.md): 問題ごとに既定・DRCR・R-DRCR を順に 1 回ずつ (同時に 1 つ)
D=analysis/drcr_20261110
for f in $(cat $1); do for c in def fold rfold; do python $D/lp_run.py run $D/miplib_lp_time.jsonl $c ${2:-120} $f; done; done
