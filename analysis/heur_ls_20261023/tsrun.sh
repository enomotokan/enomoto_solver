#!/bin/bash
# tsrun.sh <問題> <秒> <名前> [ENV=VAL ...]: run_cfg.sh と同じだが、ログの各行の頭に開始からの経過秒を付ける (終わり際の時刻を追う)
# NOLOG=1 なら ENOMOTO_MIP_LOG の代わりに ENOMOTO_MIP_END_DIAG=1 (計測と同じくログなしで、終わり際の時刻だけ)
cd "$(dirname "$0")"
p=$1; T=$2; name=$3; shift 3
mps=../../.miplib_cache/$p.mps
th=${TH:-4}
n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
if [ -n "$NOLOG" ]; then lg=ENOMOTO_MIP_END_DIAG=1; else lg=ENOMOTO_MIP_LOG=1; fi
env ENOMOTO_MIP_THREADS=$th $lg "$@" python3 -u eno_run.py $mps $T $th 2>&1 | python3 -u -c '
import sys,time
t0=time.perf_counter()
for l in sys.stdin: sys.stdout.write("[%7.2f] %s" % (time.perf_counter()-t0, l))
' > logs/${p}_${name}_${n}.log
echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log)"
