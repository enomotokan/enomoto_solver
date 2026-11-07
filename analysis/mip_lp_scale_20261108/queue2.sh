#!/bin/bash
# 改良 (11-08): momentum1・satellites2-40 を def・h で 600 秒 (種 1・2)、その後 58 問 (60 秒、種 1・2)。済んだ組は飛ばす
cd "$(dirname "$0")"
H="ENOMOTO_MIP_LPSCALE=h ENOMOTO_T_MIP_LPSCALE_MIN_RANGE=1e4 ENOMOTO_MIP_LPSCALE_LOG=1"
for s in 1 2; do for p in momentum1 satellites2-40; do for c in def h; do
  grep -q "\"problem\": \"$p\", \"cfg\": \"$c\", \"rep\": $s" big.jsonl 2>/dev/null && continue
  if [ $c = h ]; then python3 run1.py h $p 600 $s big $H; else python3 run1.py def $p 600 $s big; fi
done; done; done
python3 bench58.py 1
python3 bench58.py 2
