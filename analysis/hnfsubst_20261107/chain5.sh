#!/bin/bash
# 改良 (11-08) の計測の順 その 4: (chain4 の後) 置き換えの後の前処理のかけ直し + CUTLPCAP=c の 600 秒 (根の LP が上がった問題)
cd "$(dirname "$0")"
while pgrep -f chain4.sh >/dev/null; do sleep 20; done
python run.py re600.jsonl 600 def,subrec 1,2 neos-3656078-kumeu mzzv11 radiationm40-10-02
