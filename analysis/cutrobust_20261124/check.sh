#!/bin/bash
# 試作 (CUTVERT・CUTROBUST・CLQROW) の確かめ: MIP の fuzz 7 本 (analysis/lpiter_20261123/check.sh と同じ) を試作つきの 2 組で回し、
# cargo test を試作つき・既定で回す。リポジトリの根から: bash analysis/cutrobust_20261124/check.sh  ログは analysis/cutrobust_20261124/check_logs/
set -u
cd "$(dirname "$0")/../.."
L=analysis/cutrobust_20261124/check_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -2 $L/$name.log; }
fuzz7() {
  t=$1
  run ${t}_fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
  run ${t}_fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
  run ${t}_fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
  run ${t}_fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
  run ${t}_fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
  run ${t}_fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
  run ${t}_fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
}
( export ENOMOTO_MIP_CUTVERT=q ENOMOTO_MIP_CLQROW=1; fuzz7 A )
( export ENOMOTO_MIP_CUTROBUST=g ENOMOTO_MIP_CLQROW=2; fuzz7 B )
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (試作 A)"; ENOMOTO_MIP_CUTVERT=q ENOMOTO_MIP_CLQROW=1 $CT > $L/cargo_A.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_A.log
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
echo CHECKDONE
