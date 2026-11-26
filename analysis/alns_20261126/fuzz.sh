#!/bin/bash
# alns (ENOMOTO_MIP_ALNS=1、引数 lc で ENOMOTO_MIP_LIFTED_COVER=1 も) の誤答の確かめ (rootgap_20261121/fuzz.sh の写し)。analysis/release0110_20261120/fuzz.sh と同じ 7 本を両方つけて回し、
# cargo test を既定と両方つけた形で回す。リポジトリの根から: bash analysis/alns_20261126/fuzz.sh [lc]
# ログは analysis/alns_20261126/fuzz_logs[lc]/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/alns_20261126/fuzz_logs${1:-}; mkdir -p $L
export ENOMOTO_MIP_ALNS=1; [ "${1:-}" = lc ] && export ENOMOTO_MIP_LIFTED_COVER=1
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (alns)"; $CT > $L/cargo_alns.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_alns.log
unset ENOMOTO_MIP_ALNS ENOMOTO_MIP_LIFTED_COVER
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
