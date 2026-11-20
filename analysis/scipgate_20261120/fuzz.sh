#!/bin/bash
# 試作 ENOMOTO_MIP_VBCLQ の誤答の確かめ (scipslow_20261119/fuzz.sh の写し)。値は環境 VBCLQ (既定 e = 計測の vbce。eR = 根の LP の前の道だけ) で、
# pseudoobj が呼ばれた回・固定した回を ENOMOTO_MIP_VBCLQ_COUNT_FILE で数える (本ごとに fuzz_logs/<名>.cnt)。
# リポジトリの根から: bash analysis/scipgate_20261120/fuzz.sh   ログは analysis/scipgate_20261120/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/scipgate_20261120/fuzz_logs${VBCLQ:+_$VBCLQ}; mkdir -p $L
run() { name=$1; shift; rm -f $L/$name.cnt; echo "== $name"; ENOMOTO_MIP_VBCLQ_COUNT_FILE=$PWD/$L/$name.cnt "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatches" $L/$name.log | tail -1
  [ -f $L/$name.cnt ] && awk '{c[$3]++; if ($1>0) f[$3]++; if ($2>0) p[$3]++; s[$3]+=$1} END {for (k in c) printf "pseudoobj %s: calls %d, with fixings %d (columns %d), cutoff proved %d\n", k, c[k], f[k], s[k], p[k]}' $L/$name.cnt || echo "pseudoobj: calls 0"; }
export ENOMOTO_MIP_VBCLQ=${VBCLQ:-e}
run tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
# e の「VB01 の解が目的の最小値に届けば最適で打ち切る」道を通しやすい問題 (README 15 節。印 eopt の回がその道)
run eopt python3 analysis/scipgate_20261120/fuzz_eopt.py 2000 10000
echo FUZZ_DONE
