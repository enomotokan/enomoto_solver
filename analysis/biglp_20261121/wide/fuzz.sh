#!/bin/bash
# fix (ENOMOTO_MIP_CUTSEL_PROBE_FIT=1 + ENOMOTO_MIP_ROOTLP3=e + ENOMOTO_MIP_ROOTCAP=k) の誤答の確かめ。analysis/release0110_20261120/fuzz.sh と同じ 7 本を 3 つともつけて回し、
# cargo test を既定と 3 つともつけた形で回す。リポジトリの根から: bash analysis/biglp_20261121/wide/fuzz.sh
# ログは analysis/biglp_20261121/wide/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/biglp_20261121/wide/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_CUTSEL_PROBE_FIT=1 ENOMOTO_MIP_ROOTLP3=e ENOMOTO_MIP_ROOTCAP=k
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
echo "== cargo test (fix)"; $CT > $L/cargo_fix.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_glc.log
unset ENOMOTO_MIP_CUTSEL_PROBE_FIT ENOMOTO_MIP_ROOTLP3 ENOMOTO_MIP_ROOTCAP
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
