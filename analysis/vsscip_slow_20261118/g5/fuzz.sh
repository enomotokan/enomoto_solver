#!/bin/bash
# 群 5 の試作の誤答の確かめ (analysis/release019_20261117/fuzz.sh の組を、試作の環境変数を付けて回す)。
# リポジトリの根から: bash analysis/vsscip_slow_20261118/g5/fuzz.sh
# inh: ENOMOTO_MIP_G5INH=pc (4 スレッド)。seq: ENOMOTO_MIP_G5SEQ=1 を 1 スレッドで、大きさの下限 0・反復の上限の係数 0.05 にして小さい問題でも働かせる。
# ログは analysis/vsscip_slow_20261118/g5/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/vsscip_slow_20261118/g5/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
for k in inh seq; do
  if [ $k = inh ]; then E="ENOMOTO_MIP_G5INH=pc"; else E="ENOMOTO_MIP_G5SEQ=1 ENOMOTO_T_MIP_G5SEQ_SIZE=0 ENOMOTO_T_MIP_G5SEQ_C=0.05 ENOMOTO_MIP_THREADS=1"; fi
  run ${k}_tiny env $E ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
  run ${k}_highs env $E ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
  run ${k}_objint env $E python3 analysis/objint_20261112/fuzz_objint.py 1000 0
  run ${k}_slack env $E python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
done
