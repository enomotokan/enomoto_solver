#!/bin/bash
# presym の試作 (FP_STAGE3・FJSYNC_LABEL・POOR_NOINC・P2RIT=a) の誤答の確かめ: release019 と同じ fuzz 7 本と cargo test。
# リポジトリの根から (.venv を有効にして): bash analysis/presym_20261123/fuzz.sh
# ログは analysis/presym_20261123/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
export ENOMOTO_MIP_FP_STAGE3=1 ENOMOTO_MIP_FJSYNC_LABEL=1 ENOMOTO_MIP_POOR_NOINC=1 ENOMOTO_MIP_P2RIT=a
L=analysis/presym_20261123/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
echo "== cargo test (presym の試作)"
LD_LIBRARY_PATH=/usr/local/lib cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]' > $L/cargo_vb01.log 2>&1; echo "rc=$?"
grep "test result" $L/cargo_vb01.log | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}'
