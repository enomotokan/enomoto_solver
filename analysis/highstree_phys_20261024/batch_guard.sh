#!/bin/bash
# batch_guard.sh <名前> <回> [ENV=VAL ...]: 計測 14 で得・損の出た 9 問を 1 回ずつ (60 秒・4 スレッド、run_cfg.sh)
cd "$(dirname "$0")"
name=$1; shift; n=$1; shift
for i in $(seq 1 $n); do
for p in noswot mc11 hypothyroid-k1 dcmulti neos-960392 roll3000 physiciansched6-2 h80x6320d binkar10_1; do ./run_cfg.sh $p 60 $name "$@"; done
done
