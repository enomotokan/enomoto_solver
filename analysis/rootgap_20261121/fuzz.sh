#!/bin/bash
# glc (ENOMOTO_MIP_GEN_OLD=1 + ENOMOTO_MIP_LIFTED_COVER=1) の誤答の確かめ。analysis/release0110_20261120/fuzz.sh と同じ 7 本を両方つけて回し、
# cargo test を既定と両方つけた形で回す。リポジトリの根から: bash analysis/rootgap_20261121/fuzz.sh
# ログは analysis/rootgap_20261121/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/rootgap_20261121/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_GEN_OLD=1 ENOMOTO_MIP_LIFTED_COVER=1
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
echo "== cargo test (glc)"; $CT > $L/cargo_glc.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_glc.log
unset ENOMOTO_MIP_GEN_OLD ENOMOTO_MIP_LIFTED_COVER
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
