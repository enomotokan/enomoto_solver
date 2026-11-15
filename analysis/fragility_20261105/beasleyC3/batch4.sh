#!/bin/bash
# (a') 4 スレッド・種 4 を 3 回 (根が種 0 以外でも決まるか)。既定 (def) と試作 (rsi) を種 11〜20 で交互に
cd "$(dirname "$0")"
D="ENOMOTO_MIP_FRAG_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
R="ENOMOTO_MIP_ROBUST_LNSFREQ=1 ENOMOTO_MIP_ROBUST_IMPORT=1"
for r in 1 2 3; do python3 run1.py a4s$r 4 4 $D; done
for s in 11 12 13 14 15 16 17 18 19 20; do
  if [ $((s % 2)) -eq 0 ]; then python3 run1.py def 4 $s $D; python3 run1.py rsi 4 $s $D $R;
  else python3 run1.py rsi 4 $s $D $R; python3 run1.py def 4 $s $D; fi
done
echo BATCH4_DONE
