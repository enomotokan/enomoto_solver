#!/bin/bash
# HiGHS の uccase12 の切り分け (4 スレッド・120 秒)
cd /home/user/enomoto_solver/work_ucc; U=$PWD/../.miplib_cache/uccase12.mps
run() { name=$1; shift; python run_highs.py $U 120 "$@" > logs/$name.log 2>&1; echo "$name: $(grep -E '^ [A-Z] ' logs/$name.log | tail -1 | tr -s ' ') | $(grep RESULT logs/$name.log)"; }
run h_ucc_def_2
run h_ucc_seed1 random_seed=1
run h_ucc_seed2 random_seed=2
run h_ucc_noheur mip_heuristic_effort=0
run h_ucc_nopre presolve=off
