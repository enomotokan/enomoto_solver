#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
W=/tmp/claude-0/-home-user-enomoto-solver/9de22911-6f4d-530c-ae3b-e7d47498b968/scratchpad/runs; mkdir -p $W
while pgrep -f "run_miplib_benchmark" >/dev/null; do sleep 15; done
r() { local S=$1 T=$2 P=$3; shift 3; grep -q "\"status\"" ${T}_$P.out 2>/dev/null && return; ./run1.sh $S $P $T "$@"; }
r ours symnorm fastxgemm-n2r6s0t2 ENOMOTO_MIP_HO3_SYMNORM=1
TL=120 r ours nbdiag120 ns1830653 ENOMOTO_MIP_NB_DIAG=1
python bench58.py def,ho3 0 >> $W/bench58.log 2>&1
cp $W/bench58.jsonl $W/bench58.log .
touch q8.done
