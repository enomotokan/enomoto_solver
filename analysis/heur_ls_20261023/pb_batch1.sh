#!/bin/bash
# physbound 1 組目: 根のカットの上限を緩める診断と既存の試作 (各 1 回、60 秒)
cd "$(dirname "$0")"
P=physiciansched6-2
./run_cfg.sh $P 60 pbwide ENOMOTO_T_MIP_CUT_TIME_FRAC=0.6 ENOMOTO_T_MIP_CUT_LP_TIME_FRAC=0.1 ENOMOTO_MIP_FACE_SEP=off ENOMOTO_T_MIP_CUT_ROUNDS=100
./run_cfg.sh $P 60 pbfa ENOMOTO_MIP_FACE_ADAPT=1
./run_cfg.sh $P 60 pbext ENOMOTO_MIP_CUT_EXTEND=1
./run_cfg.sh $P 60 pbfaext ENOMOTO_MIP_FACE_ADAPT=1 ENOMOTO_MIP_CUT_EXTEND=1
