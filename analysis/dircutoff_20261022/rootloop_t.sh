#!/bin/bash
# 根のループ全体 (rootloop.sh と同じ条件) で t の形: rootloop_t.sh > rootloop_t.txt
D=$(cd $(dirname $0) && pwd)
B="ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_NO_PRESOLVE=1"
for s in highs eno_root; do
  S=ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_$s.sol
  for cfg in "t1w1 ENOMOTO_MIP_CUTSEL_DCD_T=1 ENOMOTO_MIP_CUTSEL_DCD=1" "t1w3 ENOMOTO_MIP_CUTSEL_DCD_T=1 ENOMOTO_MIP_CUTSEL_DCD=3" "traw ENOMOTO_MIP_CUTSEL_DCD_T=raw ENOMOTO_MIP_CUTSEL_DCD=1"; do
    set -- $cfg; n=$1; shift
    NODE_LIMIT=1 $D/run_eno.sh roll3000 rl_${s}_${n} 60 1 $B $S "$@" >/dev/null
    echo "$s $n | $(python3 $D/summ.py $D/logs/rl_${s}_${n}.err | cut -d'|' -f2) | rows $(grep -m1 'cut loop done' $D/logs/rl_${s}_${n}.err | sed 's/.*added, \([0-9]*\) in LP.*/\1/')"
  done
done
