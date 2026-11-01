#!/bin/bash
# HiGHS (既定・設定違い) と ENOMOTO (既定) を順に 1 本ずつ解く。リポジトリの根から実行する
D=analysis/neos960392_20261101/highs
cd "$(dirname "$0")/../../.."
for i in 1 2 3; do python $D/run_highs.py $D/highs_def_$i.log; done
python $D/run_highs.py $D/highs_norins_1.log mip_heuristic_run_rins=false
python $D/run_highs.py $D/highs_norens_1.log mip_heuristic_run_rens=false
python $D/run_highs.py $D/highs_heur0_1.log mip_heuristic_effort=0
python $D/run_highs.py $D/highs_nopre_1.log presolve=off
for i in 1 2 3 4; do ENOMOTO_MIP_LOG=1 python $D/run_ours.py 2> $D/ours_def_$i.log; done
echo ALL DONE
# 近傍を書き出す回 (近傍は 9 MB あるので置いていない。/tmp/dump/r<i>_0.mps が RENS の近傍)
for i in 1 2; do ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_DUMP=/tmp/dump/r${i}_ python $D/run_ours.py 2> $D/ours_dump_$i.log; done
# 書き出した RENS の近傍を HiGHS と ENOMOTO で解く (1 スレッド)
for d in r1 r2; do
  python -I $D/solve_dump.py /tmp/dump/${d}_0.mps $D/sub_${d}_rens_highs1t.log 1 60
  C="ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=-35.99997"
  env $C python $D/run_ours_mps.py /tmp/dump/${d}_0.mps 30 1 2> $D/sub_${d}_rens_ours_assub.log
  env $C ENOMOTO_MIP_SUBMIP_RENS=1 python $D/run_ours_mps.py /tmp/dump/${d}_0.mps 30 1 2> $D/sub_${d}_rens_ours_assub_srens.log
  env $C ENOMOTO_MIP_SUBMIP_HIGHS=1 python $D/run_ours_mps.py /tmp/dump/${d}_0.mps 30 1 2> $D/sub_${d}_rens_ours_assub_sh1.log
  ENOMOTO_MIP_LOG=1 python $D/run_ours_mps.py /tmp/dump/${d}_0.mps 30 1 2> $D/sub_${d}_rens_ours_full1t.log
done
# 外側で確かめる (2 回ずつ)
for i in 1 2; do ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_HIGHS=1 python $D/run_ours.py 2> $D/ours_sh1_$i.log; done
for i in 1 2; do ENOMOTO_MIP_LOG=1 ENOMOTO_T_MIP_NOINC_RENS_RATE=0.6 ENOMOTO_MIP_DIVE_LOCK_ORDER=1 python $D/run_ours.py 2> $D/ours_r06lock_$i.log; done
python $D/presolve_stats.py $D/highs_presolve_dev.log
