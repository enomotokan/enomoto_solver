#!/bin/bash
# 改良 (11-08): h (重い求解を見てから双子) を momentum1・satellites2-40 で 300 秒、種 1。比べに既定 (11-07 の既定、ROOTLP_SCALE を含む) と 1 (作るときから・幅 1e4 以上)
cd "$(dirname "$0")"
H="ENOMOTO_MIP_LPSCALE=h ENOMOTO_T_MIP_LPSCALE_MIN_RANGE=1e4 ENOMOTO_MIP_LPSCALE_LOG=1"
G="ENOMOTO_MIP_LPSCALE=1 ENOMOTO_T_MIP_LPSCALE_MIN_RANGE=1e4 ENOMOTO_MIP_LPSCALE_LOG=1"
for p in momentum1 satellites2-40; do
  for c in h def g; do
    grep -q "\"problem\": \"$p\", \"cfg\": \"$c\", \"rep\": 1" big.jsonl 2>/dev/null && continue
    case $c in h) python3 run1.py h $p 300 1 big $H;; g) python3 run1.py g $p 300 1 big $G;; def) python3 run1.py def $p 300 1 big;; esac
  done
done
