#!/bin/bash
# 3 回目: HiGHS の前処理後の問題 (mps/mzzv42z_hpre.mps、目的の定数項 0) を ENOMOTO・HiGHS で
cd "$(dirname "$0")"
P=mps/mzzv42z_hpre.mps
./run_cfg.sh $P 60 def
./run_highs.sh $P 60 def
./run_cfg.sh $P 60 def
./run_cfg.sh $P 60 gr ENOMOTO_MIP_GR_RESTART=1
./run_cfg.sh $P 60 cutgr ENOMOTO_MIP_CUTOFF=-20539.5 ENOMOTO_MIP_GR_RESTART=1
