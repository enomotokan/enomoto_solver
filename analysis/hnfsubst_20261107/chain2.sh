#!/bin/bash
# 改良 (11-08) の計測の順: mzzv11 の根だけ (600 秒の時間の上限、ノード 1) → traininstance6 の分け方 (600 秒)
cd "$(dirname "$0")"
ENOMOTO_BENCH_EXTRA=ENOMOTO_MIP_NODE_LIMIT=1 python run.py root2.jsonl 600 sub,subc,subac,defc,def 1 mzzv11
python run.py dec600.jsonl 600 snc,dnc,cut 1,2 traininstance6
