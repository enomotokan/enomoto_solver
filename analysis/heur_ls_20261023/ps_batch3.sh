#!/bin/bash
# physiciansched 依頼: 延長の止め方を直した後 (主のスレッドが FJ 以外から暫定解を得たら延長をやめる)。60 秒
cd "$(dirname "$0")"
MO="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
P=physiciansched6-2
for i in 1 2; do
./run_cfg.sh $P 60 pxext ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
./run_cfg.sh $P 60 pxmixocext2 ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
done
for P in beasleyC3 mc11 roll3000 neos-960392 timtab1 mzzv42z; do
./run_cfg.sh $P 60 sxext ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
./run_cfg.sh $P 60 sxmixocext2 ENOMOTO_MIP_FJ_DEBUG=1 $MO ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
done
