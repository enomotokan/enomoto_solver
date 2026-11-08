#!/bin/bash
# 改良 (11-08) の計測の順 その 3: (chain3 の後) 58 問 (60 秒、def・br、種 1・2) → traininstance6 の 1200 秒 → br の 600 秒 (ほかの当たる問題)
cd "$(dirname "$0")"
while pgrep -f chain3.sh >/dev/null; do sleep 20; done
python run.py p58b.jsonl 60 def,br 1,2 @p58
python run.py t6_1200.jsonl 1200 def,br,snc,cut 1,2 traininstance6
python run.py long600.jsonl 600 br 1,2 mzzv11 rococoB10-011000 rococoC10-001000 traininstance2
