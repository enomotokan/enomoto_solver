#!/bin/bash
# 確かめの計測 (60 秒・4 スレッド): physiciansched6-2 を F2 と AGG で 3 回ずつ交互に、ほかの 4 問を 1 回ずつ。結果は logs/batch1.txt
cd "$(dirname "$0")"
AGG="ENOMOTO_MIP_CUT_AGG=1"
F2="ENOMOTO_MIP_CUT_AGG=1 ENOMOTO_MIP_CUT_EXTEND_CATCHUP=1 ENOMOTO_T_MIP_CUT_EXTEND_MIN_GAIN=5e-4 ENOMOTO_T_MIP_CUT_EXTEND_WIN=5 ENOMOTO_MIP_SUBMIP_CUT_AGG=1 ENOMOTO_MIP_RINS_FJ_BOOST=1 ENOMOTO_MIP_FIX_CAP=1 ENOMOTO_MIP_RESTART_ALL_HEURS=1"
for i in 1 2 3; do
  ./run_cfg.sh physiciansched6-2 60 F2 $F2 >> logs/batch1.txt
  ./run_cfg.sh physiciansched6-2 60 AGG $AGG >> logs/batch1.txt
done
for p in roll3000 neos-911970 mzzv42z timtab1; do
  ./run_cfg.sh $p 60 F2 $F2 >> logs/batch1.txt
  ./run_cfg.sh $p 60 AGG $AGG >> logs/batch1.txt
done
