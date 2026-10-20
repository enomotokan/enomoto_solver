#!/bin/bash
# run.sh <in.mps> <mask> <label> : HiGHS 前処理 (presolve_rule_off=mask) → ENOMOTO(all4+pc) の FJ
S=${S:-/tmp/claude-0/-home-user-enomoto-solver/5ce3d085-c3f2-5366-a8fc-8dae8465becb/scratchpad}
R=/home/user/enomoto_solver/analysis/twovar_20261020
in=$1; mask=$2; lab=$3; out=$S/bis/h_$lab.mps
cd /tmp
python -I $R/highs_presolved_dump.py $in $out presolve_rule_off=$mask | tr '\n' ' '
PC="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_PROBE_CLIQUE=1"
bash $R/fjcheck.sh $out 6 1 $PC ${EXTRA}
