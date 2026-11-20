#!/bin/bash
# 試作 ENOMOTO_MIP_TAB_HIGHS_SIG の誤答の確かめ (release019_20261117/fuzz.sh の写し)。小さい問題でも切り替えの道を通るように
# しきい値を下げ (ENOMOTO_T_MIP_TAB_SIG_W=0.01、ほぼ全部で HiGHS の選び方)、値 1 (fill の予算を超える行が 9 割以上なら既定のまま) と値 n (fill の予算なし) で回す。
# リポジトリの根から: bash analysis/gate2_20261117/tab/fuzz.sh   ログは analysis/gate2_20261117/tab/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../../.."
export ENOMOTO_MIP_TAB_HIGHS_SIG=1 ENOMOTO_T_MIP_TAB_SIG_W=0.01
L=analysis/gate2_20261117/tab/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
export ENOMOTO_MIP_TAB_HIGHS_SIG=n
run fuzz_tiny_n env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 2000
run fuzz_highs_n env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 150
