#!/bin/bash
# traininstance6 を PROBE_REPRESOLVE=1 (prr) で 4 スレッド・600 秒、返した解を元の問題で確かめて t6prr.jsonl に足す (rep.py を使う)。
# 使い方: t6prr.sh <設定> <種の始め> <種の終わり>   設定は rep.py の形 ("K=V;K=V")
cd $(dirname $0)
python rep.py traininstance6 "$1" $2 $3 t6prr.jsonl 600 4 1
