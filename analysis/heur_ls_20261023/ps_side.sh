#!/bin/bash
# physiciansched 依頼: ENOMOTO_MIP_FJ_NOSOL_EXTEND の副作用 (FJ が予算内に解を出しにくい問題、60 秒・各 1 回)
cd "$(dirname "$0")"
MO="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
for P in beasleyC3 mc11 roll3000 neos-960392 timtab1 mzzv42z; do
./run_cfg.sh $P 60 sdef ENOMOTO_MIP_FJ_DEBUG=1
./run_cfg.sh $P 60 sext ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
./run_cfg.sh $P 60 smixoc ENOMOTO_MIP_FJ_DEBUG=1 $MO
./run_cfg.sh $P 60 smixocext2 ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
done
