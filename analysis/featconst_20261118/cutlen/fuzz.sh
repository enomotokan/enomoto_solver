#!/bin/bash
# cutlen の誤答の確かめ (analysis/cand1119_20261117/fuzz.sh の写し)。試作 ENOMOTO_MIP_CUTLEN=1 を付けて回す。
# R0=100 の組は r によらずいつも 8 ラウンドに縮める (縮めた道を必ず通す)。リポジトリの根から: bash analysis/featconst_20261118/cutlen/fuzz.sh
# ログは analysis/featconst_20261118/cutlen/fuzz_logs/ (要約だけ fuzz.txt に)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/featconst_20261118/cutlen/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; env "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
for C in "ENOMOTO_MIP_CUTLEN=1" "ENOMOTO_MIP_CUTLEN=1 ENOMOTO_T_MIP_CUTLEN_R0=100"; do
  t=$( [ "$C" = "ENOMOTO_MIP_CUTLEN=1" ] && echo a || echo b )
  run fuzz_tiny_$t $C ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
  run fuzz_highs_$t $C ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
  run fuzz_objint_$t $C python3 analysis/objint_20261112/fuzz_objint.py 1000 0
  run fuzz_slack_$t $C python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
done
