#!/bin/bash
# physiciansched 依頼: ENOMOTO_MIP_FJ_NOSOL_EXTEND=2 (解があっても混ぜる形を根のカットの終わりまで) の mixoc 2 回
cd "$(dirname "$0")"
P=physiciansched6-2
MO="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
./run_cfg.sh $P 60 psmixocext2 ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
./run_cfg.sh $P 60 psmixocext2 ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
