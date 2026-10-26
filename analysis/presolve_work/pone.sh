#!/bin/bash
# 1 問の前処理の後の形 (行・固定でない列・非零・LP 緩和の値 (HiGHS)・書き出しの md5) と前処理の時間を JSON 1 行で出す。
# 使い方: pone.sh <mps> <作業場所> <ラベル> <秒> <スレッド>  (環境変数はそのまま渡る)
mps=$1; W=$2; L=$3; T=$4; TH=$5; p=$(basename $mps .mps); D=$(cd $(dirname "$0") && pwd)
mkdir -p $W/logs; log=$W/logs/${p}_$L.log; f=$W/${p}_$L.mps; rm -f $f
ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3=$f ENOMOTO_MIP_DEBUG_STOP_PRESOLVED=1 timeout 600 python3 $D/prun.py $mps $T $TH > $log 2>&1
line=$(grep -m1 "presolve finished in" $log)
pt=$(echo "$line" | sed 's/.*finished in \([0-9.]*\)s.*/\1/'); off=$(echo "$line" | sed 's/.*offset \(.*\))/\1/')
if [ -s "$f" ]; then
  e=$(python3 $D/../presolve_size/pstats.py $f "$off"); md5=$(md5sum $f | cut -c1-12); rm -f $f
else
  e=null; md5=none; pt=null
fi
echo "{\"name\":\"$p\",\"label\":\"$L\",\"pt\":$pt,\"md5\":\"$md5\",\"e\":$e}"
