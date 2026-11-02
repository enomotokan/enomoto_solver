#!/bin/bash
# 既定・=1 (FJ だけ)・=fl (FJ と Local-MIP) を各問題 3 回ずつ交互に解く (2026-11-02)。ソルバーは同時に 1 つだけ。
# 使い方: bash run.sh 問題名...  (リポジトリの根から。結果は results.jsonl、ログは logs/)
D=analysis/feas_early_lmip_20261102
mkdir -p $D/logs
for p in "$@"; do
  mps=.miplib_cache/$p.mps
  for r in 1 2 3; do
    for mode in def fj fl; do
      case $mode in def) e=();; fj) e=(ENOMOTO_MIP_FEAS_FJ_EARLY=1);; fl) e=(ENOMOTO_MIP_FEAS_FJ_EARLY=fl);; esac
      env ENOMOTO_MIP_LOG=1 "${e[@]}" timeout 150 python3 $D/run_eno.py $mps $p $mode $r 2> $D/logs/${p}_${mode}_r$r.log >> $D/results.jsonl \
        || echo "{\"problem\": \"$p\", \"mode\": \"$mode\", \"run\": $r, \"status\": \"crash\"}" >> $D/results.jsonl
    done
  done
done
