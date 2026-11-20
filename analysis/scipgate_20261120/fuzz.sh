#!/bin/bash
# 試作 ENOMOTO_MIP_VBCLQ の誤答の確かめ (scipslow_20261119/fuzz.sh の写し)。値 1 (発見法 h と目的の上限つきの試し p の両方) で回す。
# リポジトリの根から: bash analysis/scipgate_20261120/fuzz.sh   ログは analysis/scipgate_20261120/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/scipgate_20261120/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatches" $L/$name.log | tail -1; }
export ENOMOTO_MIP_VBCLQ=1
run tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
echo FUZZ_DONE
