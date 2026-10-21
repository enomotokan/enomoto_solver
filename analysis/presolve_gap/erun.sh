#!/bin/bash
# ENOMOTO で 1 問を 4 スレッドで解く。使い方: P=<問題> T=<秒> erun.sh <名前> [環境変数=値 ...]
name=$1; shift
D=$(cd $(dirname "$0") && pwd)
P=${P:-mzzv42z}
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python3 $D/run_eno.py /home/user/enomoto_solver/.miplib_cache/$P.mps ${T:-60} > $D/logs/e_${P}_$name.log 2>&1
echo "$P $name: $(grep -E "^Presolve:" $D/logs/e_${P}_$name.log | head -1 | sed "s/.*-> //") | root $(grep -E "^Root cuts" $D/logs/e_${P}_$name.log | head -1 | sed "s/.*-> //") | $(grep RESULT $D/logs/e_${P}_$name.log) | $(grep CHECK $D/logs/e_${P}_$name.log) | warn $(grep -c "MIP: WARNING" $D/logs/e_${P}_$name.log)"
