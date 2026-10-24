#!/bin/bash
# mzzv42z 60 秒、3 回ずつ: 既定・ROWS_FIRST (rf)・ROWS_FIRST + PROBE_EQUIV_FIX (rfeq)・PROBE_EQUIV_FIX (eq)
cd $(dirname $0)
for i in 1 2 3; do
./run_cfg.sh mzzv42z 60 def
./run_cfg.sh mzzv42z 60 rf ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1
./run_cfg.sh mzzv42z 60 rfeq ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1 ENOMOTO_MIP_PROBE_EQUIV_FIX=1
./run_cfg.sh mzzv42z 60 eq ENOMOTO_MIP_PROBE_EQUIV_FIX=1
done
