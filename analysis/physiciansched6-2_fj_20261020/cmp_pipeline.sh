#!/bin/bash
# cmp_pipeline.sh <tag> [環境変数...]: 元の問題を ENOMOTO (環境変数) で前処理して書き出し (T.mps)、それに HiGHS の基本の縮約
# (規則を全部切る) をかけ (h_T.mps)、両方を ENOMOTO (pc) で前処理し直して書き出し、列名を戻して (n_T.mps / nh_T.mps) 比べる
S=${S:-/tmp/fjwork}
R=/home/user/enomoto_solver/analysis/twovar_20261020; M=/home/user/enomoto_solver/.miplib_cache/physiciansched6-2.mps
tag=$1; shift; D=$S/cmp; mkdir -p $D; cd /tmp
env "$@" ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=$D/$tag.mps timeout 100 python -I $R/eno_run.py $M 2 >/dev/null 2>&1
python -I $R/highs_presolved_dump.py $D/$tag.mps $D/h_$tag.mps presolve_rule_off=1048512 >/dev/null
PC="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_PROBE_CLIQUE=1"
for x in $tag h_$tag; do
  env $PC "$@" ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=$D/e_$x.mps timeout 100 python -I $R/eno_run.py $D/$x.mps 1 >/dev/null 2>&1
  python3 $(dirname $0)/rename.py $D/$x.mps $D/e_$x.mps $D/n_$x.mps
done
echo "ENOMOTO alone : $($(dirname $0)/trial.sh $D/n_$tag.mps ENOMOTO_MIP_NO_PRESOLVE=1 ENOMOTO_MIP_FJ_LP_REPAIR=1)"
echo "after HiGHS   : $($(dirname $0)/trial.sh $D/n_h_$tag.mps ENOMOTO_MIP_NO_PRESOLVE=1 ENOMOTO_MIP_FJ_LP_REPAIR=1)"
python3 $(dirname $0)/rowset2.py $D/n_$tag.mps $D/n_h_$tag.mps ${NSHOW:-6}
