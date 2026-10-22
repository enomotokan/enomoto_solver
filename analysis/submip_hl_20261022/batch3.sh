#!/bin/bash
# 書き出したサブ MIP を、サブ MIP の設定 (AS_SUBMIP、実際の時間上限) で 1 つずつ変えて解く (1 スレッド)
#  c25 : 根のカット 25 ラウンド・停滞 10・時間上限の 0.5
#  rr  : サブ MIP の根でダイビング式の RENS (ENOMOTO_MIP_SUBMIP_ROOT_RENS)
#  c25rr: 両方
cd "$(dirname "$0")"
S=/tmp/claude-0/-home-user-enomoto-solver/fd9e178d-c0ad-57cb-89af-579c5837bc70/scratchpad/dump
C25="ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25 ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10 ENOMOTO_T_MIP_SUBMIP_CUT_FRAC=0.5"
RR="ENOMOTO_MIP_SUBMIP_ROOT_RENS=1"
while read f c t; do
  m=$S/$f.mps
  TH=1 ./run_cfg.sh $m $t c25 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $C25
  TH=1 ./run_cfg.sh $m $t rr env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $RR
  TH=1 ./run_cfg.sh $m $t c25rr env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $C25 $RR
  TH=1 ./run_cfg.sh $m 10 c25rr10 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c $C25 $RR
done <<'L'
phys_e2_3 50625.99 2.0
phys_e2_0 50625.99 2.15
mzzv42z_1 -1192.0 16.65
roll3000_6 13951.99 3.68
neos-960392_0 -3.0 15.28
L
