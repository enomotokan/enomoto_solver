#!/bin/bash
# 試作 ENOMOTO_MIP_LU_EXTEND の確かめ: MIP の fuzz 7 本 (analysis/release0110_20261120/fuzz.sh と同じ)・LP の fuzz・Netlib 93 問を
# 試作つきで回し、cargo test を試作つき・既定で回す。リポジトリの根から: bash analysis/lpiter_20261123/check.sh
# ログは analysis/lpiter_20261123/check_logs/ (コミットしない)。Netlib は python scripts/setup_netlib_data.py で一度取得しておく。
set -u
cd "$(dirname "$0")/../.."
L=analysis/lpiter_20261123/check_logs; mkdir -p $L
export ENOMOTO_MIP_LU_EXTEND=${EXT:-1}
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run lp_fuzz python3 scripts/lp_fuzz_vs_highs.py 1000 simplex
run netlib python3 -m enomoto_solver.benchmark_highs --max-vars 100000 --out $L/netlib.csv
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (LU_EXTEND)"; $CT > $L/cargo_ext.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_ext.log
unset ENOMOTO_MIP_LU_EXTEND
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
