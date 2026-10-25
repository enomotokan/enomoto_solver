#!/bin/bash
# ENOMOTO の前処理後 (クリーク行の拡張の後) の形と LP 緩和の値を出す。環境変数はそのまま渡る。
# 前処理後の問題を書き出したところで止める (短い時間制限で解くと前処理のプロービングが時間で打ち切られ、形が揺れる)。
# 使い方: eone.sh <mps> <作業場所> <ラベル>
mps=$1; W=$2; L=$3; p=$(basename $mps .mps); D=$(cd $(dirname "$0") && pwd)
log=$W/${p}_$L.log; f=$W/${p}_$L.mps; rm -f $f
ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=$f python3 $D/run1.py $mps > $log 2>&1 &
pid=$!
while kill -0 $pid 2>/dev/null; do grep -q "presolved problem written to $f" $log && break; sleep 0.2; done
kill $pid 2>/dev/null; wait $pid 2>/dev/null
eoff=$(grep -m1 "presolved problem written to $f" $log | sed 's/.*offset \(.*\))/\1/')
et=$(grep -m1 "^Presolve:" $log | sed 's/.*(\(.*\)s)/\1/')
e=$( [ -n "$eoff" ] && python3 $D/pstats.py $f "$eoff" || echo null)
echo "{\"name\":\"$p\",\"label\":\"$L\",\"et\":\"$et\",\"e\":$e}"
