#!/bin/bash
# 誤答の検査 (1 スレッド。wheel は tu2v5 = e 入り)。使い方: run_fuzz.sh <出力のディレクトリ>  (終わったら fuzz/ に写す)
# (1 回目は低い優先度で計測と並べて回したが、途中の git rebase でこのディレクトリが作り直されて出力を失ったので、計測の後に回し直した)
OUT=$1
WT=/home/user/enomoto_solver
export PYTHONPATH=/home/user/pkgs/tu2v5 ENOMOTO_MIP_THREADS=1
for v in baes bes 1 baev; do
  ENOMOTO_MIP_IMPLINT_TU2=$v python3 $WT/analysis/implint_tu_20261110/fuzz_tu.py 3000 0 > $OUT/fuzz_tu_$v.out 2>&1
done
python3 $WT/analysis/implint_tu_20261110/fuzz_tu.py 3000 0 > $OUT/fuzz_tu_def.out 2>&1
for v in baes bes; do
  (cd $WT/benchmarks/equiv2_bug_20261022 && ENOMOTO_MIP_IMPLINT_TU2=$v python3 fuzz_tiny.py 400 0) > $OUT/fuzz_tiny_$v.out 2>&1
  (cd $WT && ENOMOTO_MIP_IMPLINT_TU2=$v ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300) > $OUT/fuzz_vs_highs_$v.out 2>&1
done
# 既定 (環境変数なし。空の値は「付けた」扱いになるので付けない)
(cd $WT/benchmarks/equiv2_bug_20261022 && python3 fuzz_tiny.py 400 0) > $OUT/fuzz_tiny_def.out 2>&1
(cd $WT && ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300) > $OUT/fuzz_vs_highs_def.out 2>&1
echo FUZZ_DONE
