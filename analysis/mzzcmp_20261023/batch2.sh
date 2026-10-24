#!/bin/bash
# 2 回目: HiGHS 150 秒 (証明の経過)、打ち切り値つき ENOMOTO の再スタートの扱い
cd "$(dirname "$0")"
C="ENOMOTO_MIP_CUTOFF=-20539.5"
./run_highs.sh mzzv42z 150 def150
./run_cfg.sh mzzv42z 60 cutgr $C ENOMOTO_MIP_GR_RESTART=1
./run_cfg.sh mzzv42z 60 cutnr $C ENOMOTO_MIP_NO_RESTART=1
./run_cfg.sh mzzv42z 150 cut150 $C
