#!/bin/bash
# run_cfg.sh <問題> <秒> <名前> [環境変数...]: ENOMOTO (4 スレッド) を 1 回回して logs/<問題>_<名前>_<秒>_<n>.log に書き、要約を出す
P=$1; T=$2; N=$3; shift 3
D=$(cd "$(dirname "$0")" && pwd); M=/home/user/enomoto_solver/.miplib_cache/$P.mps
k=1; while [ -f $D/logs/${P}_${N}_${T}_$k.log ]; do k=$((k+1)); done
L=$D/logs/${P}_${N}_${T}_$k.log
cd /tmp; env "$@" ENOMOTO_MIP_THREADS=4 timeout $((T+120)) python -I /home/user/enomoto_solver/analysis/twovar_20261020/eno_run.py $M $T > $L 2>&1
pre=$(grep "^Presolve:" $L | head -1 | sed 's/.*-> //')
res=$(grep RESULT $L | sed "s/.*'status': '\([a-z_]*\)', 'objective': \([^,]*\), 'best_bound': \([^,]*\),.*'nodes': \([0-9]*\),.*'root_bound': \([^,]*\), 'first_solution_time': \([^,]*\), 'first_solution_objective': \([^,]*\),.*/st=\1 obj=\2 bound=\3 nodes=\4 root=\5 first=\7@\6/")
echo "$P $N run$k: pre=[$pre] $res"
