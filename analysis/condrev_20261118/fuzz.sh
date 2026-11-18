#!/bin/bash
# condrev の試作の誤答の確かめ: analysis/release019_20261117/fuzz.sh と同じ 7 本を、3 件の試作を全部入れて回す。
# sbt は条件を通る問題と通らない問題が両方出るよう ENOMOTO_MIP_SBT_GENINT_MIN=1 (一般整数の列が 1 本でもあれば sbt、なければ切る)。
# リポジトリの根から: bash analysis/condrev_20261118/fuzz.sh 。ログは analysis/condrev_20261118/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/condrev_20261118/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_SB_TIGHTEN=1 ENOMOTO_MIP_SB_TIGHTEN_INHERIT=1 ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR=1 ENOMOTO_MIP_SB_POLY_RC=1 ENOMOTO_MIP_SBT_GENINT_MIN=1
export ENOMOTO_T_MIP_SUBFIX_CAP=0.05 ENOMOTO_MIP_ROOTSTOP=rb ENOMOTO_T_MIP_ROOTSTOP_RATE=0.02
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
