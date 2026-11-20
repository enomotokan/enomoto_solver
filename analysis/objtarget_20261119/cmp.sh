#!/bin/sh
# OBJTARGET 3・5: 対象 (解探しが弱い) 4 問と下界側が弱い 2 問 × 種 1・2 × {def, l, f, tpre}、60 秒、4 スレッド。res_traj_C_<設定>.jsonl
cd "$(dirname "$0")"
PROBS="neos-5107597-kakapo comp21-2idx rococoB10-011000 neos-3024952-loue gen-ip002 assign1-5-8"
for s in 1 2; do
  python traj.py C_def 60 $s $PROBS
  ENOMOTO_MIP_OBJTARGET=l python traj.py C_l 60 $s $PROBS
  ENOMOTO_MIP_OBJTARGET=f python traj.py C_f 60 $s $PROBS
  ENOMOTO_MIP_TPRE=b python traj.py C_tpre 60 $s $PROBS
done
