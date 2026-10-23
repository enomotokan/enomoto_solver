#!/bin/bash
# physiciansched 依頼: 既定・mixoc と ENOMOTO_MIP_FJ_NOSOL_EXTEND の比較 (60 秒、この順で回した)
cd "$(dirname "$0")"
P=physiciansched6-2
MO="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
./run_cfg.sh $P 60 psdef ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh $P 60 psext ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
./run_cfg.sh $P 60 psmixoc ENOMOTO_MIP_FJ_DEBUG=1 $MO
./run_cfg.sh $P 60 psmixocext ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
./run_cfg.sh $P 60 psmixocext ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
