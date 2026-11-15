#!/bin/bash
# mc11_wait1 の本番 3: neos-911970 の追加 (def・f1s をあと 2 回、停滞の判定との関係を見る f1s_l・def_l を 2 回) と 58 問 (def・f1s を 1 回ずつ)
cd "$(dirname "$0")"
python run.py neos-911970 def,f1s,f1s_l,def_l 2 2
python bench58.py def,f1s 0
