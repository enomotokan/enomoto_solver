#!/bin/bash
# F3 の計測 (60 秒・4 スレッド): physiciansched6-2 を 3 回、ほかの 4 問を 1 回ずつ。結果は logs/batch2.txt
cd "$(dirname "$0")"
F3="ENOMOTO_MIP_CUT_AGG=1 ENOMOTO_MIP_RESTART_ALL_HEURS=1 ENOMOTO_MIP_RESTART_REF=1 ENOMOTO_MIP_SUBMIP_CUT_AGG=1 ENOMOTO_MIP_RINS_ROOT_BOOST=1"
for i in 1 2 3; do ./run_cfg.sh physiciansched6-2 60 F3 $F3 >> logs/batch2.txt; done
for p in roll3000 neos-911970 mzzv42z timtab1; do ./run_cfg.sh $p 60 F3 $F3 >> logs/batch2.txt; done
