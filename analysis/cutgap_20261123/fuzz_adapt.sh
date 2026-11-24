#!/bin/bash
# 試作 ENOMOTO_MIP_ENUM_ADAPT=1 (8 節) の誤答の確かめ。analysis/rootgap_20261121/fuzz.sh と同じ 7 本を ENUM_ADAPT=1 で回し、cargo test を既定と ENUM_ADAPT=1 で回す。
# リポジトリの根から: bash analysis/cutgap_20261123/fuzz_adapt.sh   ログは analysis/cutgap_20261123/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/cutgap_20261123/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_ENUM_ADAPT=1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_ENUM_ADAPT=1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint env ENOMOTO_MIP_ENUM_ADAPT=1 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack env ENOMOTO_MIP_ENUM_ADAPT=1 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_ENUM_ADAPT=1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 env ENOMOTO_MIP_ENUM_ADAPT=1 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 env ENOMOTO_MIP_ENUM_ADAPT=1 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
echo "== cargo test (ENUM_ADAPT=1)"; ENOMOTO_MIP_ENUM_ADAPT=1 $CT > $L/cargo_adapt.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_adapt.log
