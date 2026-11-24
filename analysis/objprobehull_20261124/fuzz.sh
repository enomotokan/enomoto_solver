#!/bin/bash
# objprobehull の試作 (ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL=1) の誤答の確かめ: heuradapt と同じ fuzz 7 本と cargo test。
# fuzz の小さい問題では既定の見切り (候補 500 本以上・ギャップ 5% 以下) で根の打ち切り値つきプロービングが走らないので、
# 既存のつまみで見切りを外して (MIN_CANDS=0・MAX_GAP=1) 試作の締め付けが実際に入る形で回す。
# リポジトリの根から: bash analysis/objprobehull_20261124/fuzz.sh
# ログは analysis/objprobehull_20261124/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
export ENOMOTO_MIP_ROOT_OBJ_PROBE_HULL=1 ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MIN_CANDS=0 ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MAX_GAP=1
L=analysis/objprobehull_20261124/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
echo "== cargo test (objprobehull の試作)"
LD_LIBRARY_PATH=/usr/local/lib cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]' > $L/cargo_test.log 2>&1; echo "rc=$?"
grep "test result" $L/cargo_test.log | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}'
