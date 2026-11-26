#!/bin/bash
# T1PRIMAL (ENOMOTO_MIP_T1PRIMAL=1) の誤答の確かめ。analysis/t1gap_20261121/mix/fuzz.sh と同じ 7 本を 1 スレッドで回し、cargo test を既定で回す。
# fuzz の問題は小さく、そのままでは貯え (Local-MIP の 1 切れ 0.2 秒) がたまる前に解き終わるので、1 切れを 0.002 秒にして試作の道を通す。
# リポジトリの根から: bash analysis/t1primal_20261126/fuzz.sh。ログは analysis/t1primal_20261126/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/t1primal_20261126/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_T1PRIMAL=1 ENOMOTO_T_MIP_FJ_MIX_SLICE=0.002
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
unset ENOMOTO_MIP_THREADS ENOMOTO_MIP_T1PRIMAL ENOMOTO_T_MIP_FJ_MIX_SLICE
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
