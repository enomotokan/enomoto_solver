#!/bin/bash
# Feasibility Pump の比べ (fp.py)。結果は logs/fp_<名前>.txt と logs/fp_summary.txt
cd "$(dirname "$0")"
run() { name=$1; shift; python3 fp.py "$@" > logs/fp_$name.txt 2>&1; tail -n 3 logs/fp_$name.txt | grep -E "found|stopped|bound" >> logs/fp_summary.txt; }
run b1_eno_s1 dump/b1_0 eno "" 300 300 1
run b1_std_s1 dump/b1_0 std "" 500 300 1
run b1_std_s2 dump/b1_0 std "" 500 300 2
run b1_dist_s1 dump/b1_0 dist "" 500 300 1
run b1_std_z51029 dump/b1_0 std 51029 500 300 1
run b1_std_z49324 dump/b1_0 std 49324 500 300 1
run a1_eno_s1 dump/a1_0 eno "" 300 300 1
run a1_std_s1 dump/a1_0 std "" 500 300 1
run a1_std_z49324 dump/a1_0 std 49324 500 300 1
