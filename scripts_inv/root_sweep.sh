#!/bin/bash
# 使い方: root_sweep.sh <名前> [ENV=VAL ...]   根だけ (ノード上限 1、時間上限 60 秒) を回して根のカット後の値を出す
name=$1; shift
out=/home/user/wt/roll3000/runs/root_$name.log
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" /tmp/with_slot.sh python -c "
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path('/home/user/enomoto_solver/.miplib_cache/roll3000.mps')); m, _, _ = bh._build_our_model(lp)
r = m.solve(None, False, 60, None, 1, 4, log=True)
print('RESULT', r['objective'], r['best_bound'], r['root_bound'])
" > $out 2>&1
echo "$name: $(grep 'Root cuts' $out | head -1) rounds=$(grep -c 'cut round' $out) | $(grep RESULT $out)"
