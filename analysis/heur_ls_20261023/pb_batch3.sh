#!/bin/bash
# physbound 3 組目: 既定・mixoc と CUT_STRETCH (と FJ_NOSOL_EXTEND) を各 2 回、副作用の 3 問を各 1 回 (60 秒)
cd "$(dirname "$0")"
P=physiciansched6-2
MX="ENOMOTO_MIP_FJ_LMIP_MIX=1 ENOMOTO_MIP_SYM_ORIG_CUT=1"
ST="ENOMOTO_MIP_CUT_STRETCH=1"
for i in 1 2; do
  ./run_cfg.sh $P 60 k_def
  ./run_cfg.sh $P 60 k_st $ST
  ./run_cfg.sh $P 60 k_stx1 $ST ENOMOTO_MIP_FJ_NOSOL_EXTEND=1
  ./run_cfg.sh $P 60 k_mx $MX
  ./run_cfg.sh $P 60 k_mxst $MX $ST
  ./run_cfg.sh $P 60 k_mxstx2 $MX $ST ENOMOTO_MIP_FJ_NOSOL_EXTEND=2
done
for q in neos-911970 mzzv42z roll3000; do
  ./run_cfg.sh $q 60 k_def
  ./run_cfg.sh $q 60 k_st $ST
done
