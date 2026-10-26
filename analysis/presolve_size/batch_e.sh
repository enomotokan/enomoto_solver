#!/bin/bash
# 55 問で ENOMOTO の前処理後の形を設定ごとに出す。使い方: batch_e.sh <ラベル> <出力> (環境変数はそのまま渡る)
D=$(cd $(dirname "$0") && pwd); W=${W:-/tmp/presolve_size}; mkdir -p $W
for m in $D/../../.miplib_cache/*.mps; do $D/eone.sh $m $W $1 >> $D/$2; done
