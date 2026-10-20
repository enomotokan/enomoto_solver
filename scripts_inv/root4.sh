#!/bin/bash
# 使い方: root4.sh <名前> <問題> [ENV=VAL ...]   根だけ (ノード上限 1、時間上限 60 秒、4 スレッド) を回して runs4/root_<名前>.log に書く
name=$1; prob=$2; shift 2
out=/home/user/enomoto_solver/runs4/root_$name.log
env PYTHONPATH=${PYLIB:-} ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python -c "
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path('/home/user/enomoto_solver/.miplib_cache/$prob.mps')); m, _, _ = bh._build_our_model(lp)
r = m.solve(None, False, 60, None, 1, 4, log=True)
print('RESULT', r['objective'], r['best_bound'], r['root_bound'])
" > $out 2>&1
echo "$name: $(grep -m1 'Root cuts' $out | cut -c1-120) | $(grep RESULT $out)"
