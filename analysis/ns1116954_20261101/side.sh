#!/bin/bash
# 損の出やすい 6 問を既定 (def) と試作 (early = ENOMOTO_MIP_FEAS_FJ_EARLY=1) で交互に 2 回ずつ解く (4 スレッド・60 秒、1 つずつ)
D=$(cd "$(dirname "$0")" && pwd); C=$D/../../.miplib_cache
for r in 1 2; do for p in neos-960392 physiciansched6-2 h80x6320d qiu 10teams n5-3; do
  for v in def early; do
    if [ $v = early ]; then export ENOMOTO_MIP_FEAS_FJ_EARLY=1; else unset ENOMOTO_MIP_FEAS_FJ_EARLY; fi
    out=$(ENOMOTO_MIP_LOG=1 timeout 200 python "$D/run_eno.py" "$C/$p.mps" 2> "$D/logs/side/${p}_${v}_r$r.log")
    echo "{\"problem\": \"$p\", \"variant\": \"$v\", \"run\": $r, \"result\": ${out:-null}}" | tee -a "$D/side.jsonl"
  done
done; done
