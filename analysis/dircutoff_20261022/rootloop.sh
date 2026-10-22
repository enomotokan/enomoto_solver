#!/bin/bash
# 根のループ全体 (roll3000、前処理なし、1 スレッド、根だけ、裏のループなし)。DCD の向き先は与えた解 (最初のラウンドから)
# rootloop.sh > rootloop.txt
D=$(cd $(dirname $0) && pwd)
B="ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_NO_PRESOLVE=1"
for s in highs eno_root; do
  S=ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_$s.sol
  for i in 1 2; do
    for cfg in "def" "dcd ENOMOTO_MIP_CUTSEL_DCD=1" "keep ENOMOTO_MIP_CUT_KEEP_DCD=1" "dcdkeep ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUT_KEEP_DCD=1" "prod0keep ENOMOTO_MIP_CUTSEL_DCD_PROD=1 ENOMOTO_T_CUTSEL_DCD_DENS_EXP=0 ENOMOTO_MIP_CUT_KEEP_DCD=1"; do
      set -- $cfg; n=$1; shift
      [ $n = def ] && [ $s = eno_root ] && continue
      NODE_LIMIT=1 $D/run_eno.sh roll3000 rl_${s}_${n}_$i 60 1 $B $S "$@" >/dev/null
      echo "$s $n $i | $(python3 $D/summ.py $D/logs/rl_${s}_${n}_$i.err | cut -d'|' -f2) | rows $(grep -m1 'cut loop done' $D/logs/rl_${s}_${n}_$i.err | sed 's/.*added, \([0-9]*\) in LP.*/\1/')"
    done
  done
done
