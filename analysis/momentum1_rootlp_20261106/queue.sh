#!/bin/bash
# 計測の待ち行列 (1 本ずつ): momentum1 600 秒 (base・fix)、似た問題 300 秒 (base・fix)、58 問 2 回
cd "$(dirname "$0")/../.."
D=analysis/momentum1_rootlp_20261106
python3 $D/run1.py base momentum1 600 0 long ENOMOTO_MIP_ROOTLP_SCALE_RESCUE=0
python3 $D/run1.py fix momentum1 600 0 long ENOMOTO_MIP_ROOTLP_SCALE=1 ENOMOTO_SIMPLEX_SCALE_LOG=1
for p in supportcase42 uccase12 opm2-z10-s4 dano3_3 satellites2-40 rail01; do
  python3 $D/run1.py base $p 300 0 sim ENOMOTO_MIP_ROOTLP_SCALE_RESCUE=0
  python3 $D/run1.py fix $p 300 0 sim ENOMOTO_MIP_ROOTLP_SCALE=1 ENOMOTO_SIMPLEX_SCALE_LOG=1
done
python3 $D/bench58.py 0
python3 $D/bench58.py 1
echo QUEUE_DONE
