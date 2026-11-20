#!/bin/bash
# gate2 候補 2 の誤答の確かめ: analysis/release019_20261117/fuzz.sh と同じ組を `ENOMOTO_MIP_CLQPRB=1dgrs` で回す (cargo test は別に回した)。
# リポジトリの根から: bash analysis/gate2_20261117/clqprb_s/fuzz.sh 。ログは analysis/gate2_20261117/clqprb_s/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../../.."
export ENOMOTO_MIP_CLQPRB=1dgrs
L=analysis/gate2_20261117/clqprb_s/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
