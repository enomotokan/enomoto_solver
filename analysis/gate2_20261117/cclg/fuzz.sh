#!/bin/bash
# gate2 候補 8 (cclg) の誤答の確かめ: release019 の fuzz.sh と同じ組 + cutconf の fuzz を、試作の環境変数つきで回す。cargo test も (既定)。
# リポジトリの根から: bash analysis/gate2_20261117/cclg/fuzz.sh 。ログは analysis/gate2_20261117/cclg/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/gate2_20261117/cclg/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_CUTCONF=1 ENOMOTO_MIP_CUTCONF_LGATE=${LG:-25,5,20}
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_cutconf python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 3000
# 割合を小さくして捨てる側の道も通す (L0 = 5)
run fuzz_cutconf_lg5 env ENOMOTO_MIP_CUTCONF_LGATE=5,2,5 python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 4000
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_MIP_CUTCONF_LGATE=5,2,5 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
unset ENOMOTO_MIP_CUTCONF ENOMOTO_MIP_CUTCONF_LGATE
echo "== cargo test (既定)"
LD_LIBRARY_PATH=/usr/local/lib cargo test --release --config 'target.x86_64-unknown-linux-gnu.rustflags=["-C","link-arg=-L/usr/local/lib","-C","link-arg=-lpython3.13"]' > $L/cargo.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo.log
