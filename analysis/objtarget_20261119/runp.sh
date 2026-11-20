#!/bin/sh
# OBJTARGET 8 の試作 (ENOMOTO_MIP_TPRE=p、PDLP の点からの FJ を開始解に) と def の比べ。60 秒、4 スレッド、種 1・2。
cd "$(dirname "$0")"
PP="satellites2-60-fs satellites2-40 academictimetablesmall trento1 neos-5107597-kakapo cvs16r128-89 mzzv42z neos-1456979"
for s in 1 2; do
  python traj.py E_def 60 $s $PP
  ENOMOTO_MIP_TPRE=p python traj.py E_p 60 $s $PP
done
