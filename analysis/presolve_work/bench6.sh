#!/bin/bash
# 6 章の計測: 16 問を既定 (pwdef) と PRESOLVE_WORK (pwwork) で 4 回ずつ、順を入れ替えて (60 秒・4 スレッド)。
# 使い方: ENOMOTO_BENCH_SHA=<ビルドしたコミット> bench6.sh
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_ONLY="neos-960392 mc11 noswot roll3000 physiciansched6-2 hypothyroid-k1 dcmulti h80x6320d binkar10_1 drayage-100-23 drayage-25-23 mzzv42z neos-950242 ns1116954 nursesched-sprint02 nw04"
B=analysis/heur_ls_20261023/bench_mix.sh
for k in "def HLs1" "work HLs1" "work HLs2" "def HLs2" "def HLs3" "work HLs3" "work HLs4" "def HLs4"; do
  set -- $k
  if [ $1 = work ]; then $B pwwork $2 ENOMOTO_MIP_PRESOLVE_WORK=1; else $B pwdef $2; fi
done
echo BENCH6 ALL DONE
