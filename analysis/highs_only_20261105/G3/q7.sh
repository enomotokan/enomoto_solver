#!/bin/bash
# 再起動で止まった回をやり直す (済んだ .out に "status" があれば飛ばす)
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
while pgrep -f "run_miplib_benchmark" >/dev/null; do sleep 15; done
r() { local S=$1 T=$2 P=$3; shift 3; grep -q "\"status\"" ${T}_$P.out 2>/dev/null && return; ./run1.sh $S $P $T "$@"; }
r highs hi rococoC10-001000
r ours def rococoC10-001000
r ours symnorm fastxgemm-n2r6s0t2 ENOMOTO_MIP_HO3_SYMNORM=1
r ours tr ns1830653 ENOMOTO_MIP_TREE_RESTART=1
r ours icq rococoC10-001000 ENOMOTO_MIP_TREE2_INTAGG=g ENOMOTO_MIP_CLIQUE_CUTS=on
TL=120 r ours nbdiag120 ns1830653 ENOMOTO_MIP_NB_DIAG=1
python bench58.py def,ho3 0 >> bench58.log 2>&1
touch q7.done
