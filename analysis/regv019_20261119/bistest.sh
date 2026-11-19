#!/bin/bash
# 二分探索の 1 手: 作業木の今のコミットをビルドして入れ、neos5 を 1 スレッド 300 秒で解く。130 秒超 (または時間切れ) なら bad (rc 1)
S=/tmp/claude-0/-home-user-enomoto-solver/cf852a59-a863-5581-8128-d62d96d07c4c/scratchpad
cd $S/bis || exit 125
export CARGO_TARGET_DIR=$S/bistarget
rm -rf $S/wheels; $S/bisenv/bin/maturin build --release -i $S/bisenv/bin/python -o $S/wheels -q >/dev/null 2>&1 || exit 125
$S/bisenv/bin/pip install -q --force-reinstall --no-deps $S/wheels/*.whl >/dev/null 2>&1 || exit 125
out=$(env -u ENOMOTO_MIP_LOG ENOMOTO_MIP_THREADS=1 $S/bisenv/bin/python /home/user/enomoto_solver/scripts/run_miplib_benchmark.py --worker-ours /home/user/enomoto_solver/.miplib_cache/neos5.mps --time-limit 300 --threads 1 2>/dev/null | grep '^{' | tail -1)
t=$(echo "$out" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['time'] if d['status']=='optimal' else 999)")
echo "$(git rev-parse --short HEAD) neos5 $t $(echo "$out" | python3 -c "import json,sys; print(json.load(sys.stdin)['nodes'])")" >> $S/bislog.txt
python3 -c "import sys; sys.exit(1 if $t > 130 else 0)"
