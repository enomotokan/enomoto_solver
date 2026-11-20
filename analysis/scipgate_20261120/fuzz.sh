#!/bin/bash
# 試作 ENOMOTO_MIP_VBCLQ の誤答の確かめ (scipslow_20261119/fuzz.sh の写し)。計測の vbcl と同じ VB01=b + VBCLQ=l で回し、
# pseudoobj が呼ばれた回・固定した回を ENOMOTO_MIP_VBCLQ_COUNT_FILE で数える (本ごとに fuzz_logs/<名>.cnt)。
# リポジトリの根から: bash analysis/scipgate_20261120/fuzz.sh   ログは analysis/scipgate_20261120/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/scipgate_20261120/fuzz_logs; mkdir -p $L
run() { name=$1; shift; rm -f $L/$name.cnt; echo "== $name"; ENOMOTO_MIP_VBCLQ_COUNT_FILE=$PWD/$L/$name.cnt "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatches" $L/$name.log | tail -1
  [ -f $L/$name.cnt ] && awk '{c++; if ($1>0) f++; if ($2>0) p++; s+=$1} END {printf "pseudoobj: calls %d, with fixings %d (columns %d), cutoff proved %d\n", c, f, s, p}' $L/$name.cnt || echo "pseudoobj: calls 0"; }
export ENOMOTO_MIP_VB01=b ENOMOTO_MIP_VBCLQ=l
run tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
echo FUZZ_DONE
