#!/bin/bash
# 書き出したサブ MIP: 強分岐なし (NO_SB2) を足す (1 スレッド、AS_SUBMIP)
#  nsb     : 強分岐なしだけ (実際の時間上限)
#  rrnsb   : 根の RENS + 強分岐なし (実際の時間上限)
#  rrnsb10 : 同、10 秒
#  c25rrnsb10 : カット 25 ラウンド + 根の RENS + 強分岐なし、10 秒
#  nsb60   : 強分岐なしだけ、60 秒
cd "$(dirname "$0")"
S=/tmp/claude-0/-home-user-enomoto-solver/fd9e178d-c0ad-57cb-89af-579c5837bc70/scratchpad/dump
C25="ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25 ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10 ENOMOTO_T_MIP_SUBMIP_CUT_FRAC=0.5"
RR="ENOMOTO_MIP_SUBMIP_ROOT_RENS=1"
NSB="ENOMOTO_MIP_SUBMIP_NO_SB2=1"
while read f c t; do
  m=$S/$f.mps
  TH=1 ./run_cfg.sh $m $t nsb env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $NSB
  TH=1 ./run_cfg.sh $m $t rrnsb env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $RR $NSB
  TH=1 ./run_cfg.sh $m 10 rrnsb10 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $RR $NSB
  TH=1 ./run_cfg.sh $m 10 c25rrnsb10 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $C25 $RR $NSB
  TH=1 ./run_cfg.sh $m 60 nsb60 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $NSB
done <<'L'
phys_e2_3 50625.99 2.0
phys_e2_0 50625.99 2.15
mzzv42z_1 -1192.0 16.65
roll3000_6 13951.99 3.68
neos-960392_0 -3.0 15.28
L
