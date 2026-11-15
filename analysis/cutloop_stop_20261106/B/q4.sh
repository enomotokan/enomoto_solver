#!/bin/bash
# 計測の待ち行列 4: q3 の後に、ラウンドを延ばさない版 csn (ENOMOTO_MIP_CUTSTALL_GAP=1n) で 58 問を 2 回 (rep 1・2)。def は q2 の結果と比べる
cd $(dirname $0)
until grep -q "q3 done" q3.log; do sleep 30; done
python run58.py 1 csn
python run58.py 2 csn
echo q4 done
