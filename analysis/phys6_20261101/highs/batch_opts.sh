#!/bin/bash
# HiGHS の設定を 1 つずつ変えて解く (各 2 回、4 スレッド・60 秒)。logs/h_<名前>_<n>.{log,trace}、結果は logs/opts.out
cd "$(dirname "$0")"
M=../../../.miplib_cache/physiciansched6-2.mps
run() { name=$1; shift; for i in 1 2; do echo "$name $i $(python highs_log.py $M 60 logs/h_${name}_$i "$@" | tail -1)" >> logs/opts.out; done; }
run norens mip_heuristic_run_rens=false
run norins mip_heuristic_run_rins=false
run nopre presolve=off
run norestart mip_allow_restart=false
run nofj mip_heuristic_run_feasibility_jump=false
run heur0 mip_heuristic_effort=0
run nosym mip_detect_symmetry=false
run nolift mip_lifting_for_probing=0
