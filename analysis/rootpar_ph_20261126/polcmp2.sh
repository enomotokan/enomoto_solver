#!/bin/bash
# 研磨の形の比べ 2 (親の依頼、e の案を足す): dano3_3・nattai × d・w・i・ij・j・e・ef、4 スレッド 120 秒、種 1〜3。順に 1 本ずつ
# e = POLISH_WARM=e (研磨しない、見積りだけ)、ef = POLISH_WARM=e + ROOTPAR_PH=f (木の中は見積り、返す前に 1 回だけ研磨)
# 比べ 3 (版 93d866ccb、ROOTPAR_CUT・PARIDLE・UNIFY_FIX の既定化の後): OUT=res_polcmp3.jsonl CFGS="d e eg egj" TAG=v3
cd "$(dirname "$0")"
PY=../../.venv/bin/python
OUT=${OUT:-res_polcmp2.jsonl}
for seed in ${SEEDS:-1 2 3}; do
for prob in ${PROBS:-dano3_3 neos-5188808-nattai}; do
  for c in ${CFGS:-d w i ij j e ef}; do
    case $c in
      d) e=();; e) e=(ENOMOTO_MIP_POLISH_WARM=e);; ef) e=(ENOMOTO_MIP_POLISH_WARM=e ENOMOTO_MIP_ROOTPAR_PH=f);;
      eg) e=(ENOMOTO_MIP_POLISH_WARM=e ENOMOTO_MIP_ROOTPAR_PH=g);; egj) e=(ENOMOTO_MIP_POLISH_WARM=e ENOMOTO_MIP_ROOTPAR_PH=gj);;
      *) e=(ENOMOTO_MIP_ROOTPAR_PH=$c);;
    esac
    ENOMOTO_T_MIP_SEED=$seed $PY polcmp.py $OUT $prob ${TH:-4} ${TL:-120} $c${TAG:+_$TAG} ENOMOTO_MIP_LOG=1 "${e[@]}"
  done
done
done
