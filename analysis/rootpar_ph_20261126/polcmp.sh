#!/bin/bash
# 研磨の作り直しの比べ (4 問 × d・w・i・j・ij、4 スレッド 120 秒、種 1)。順に 1 本ずつ回す
# d = 既定、w・i・j・ij = ENOMOTO_MIP_ROOTPAR_PH の文字 (README 5 節)
cd "$(dirname "$0")"
PY=../../.venv/bin/python
OUT=${OUT:-res_polcmp4.jsonl}
TH=${TH:-4}
for prob in ${PROBS:-dano3_3 neos-5188808-nattai swath1 supportcase7}; do
  for c in ${CFGS:-d w i j ij}; do
    if [ "$c" = d ]; then e=(); else e=(ENOMOTO_MIP_ROOTPAR_PH=$c); fi
    $PY polcmp.py $OUT $prob $TH ${TL:-120} $c ENOMOTO_MIP_LOG=1 "${e[@]}"
  done
done
