#!/bin/bash
# 書き出したサブ MIP を単独で解き比べる (1 スレッド)。
#  hi   : HiGHS 1 スレッド・60 秒・objective_bound = 打ち切り値
#  sub  : ENOMOTO、サブ MIP と同じ設定 (AS_SUBMIP)、時間は実際のサブ MIP と同じ
#  sub60: ENOMOTO、サブ MIP と同じ設定、60 秒 (ノード上限 500 のまま)
#  solo : ENOMOTO、単独の求解 (1 スレッド・60 秒・打ち切り値つき)
cd "$(dirname "$0")"
S=/tmp/claude-0/-home-user-enomoto-solver/fd9e178d-c0ad-57cb-89af-579c5837bc70/scratchpad/dump
# 名前 打ち切り値 実際の時間上限
while read f c t; do
  m=$S/$f.mps
  n=1; while [ -e logs/sub_${f}_hi_${n}.log ]; do n=$((n+1)); done
  python3 highs_run.py $m 60 1 $c > logs/sub_${f}_hi_${n}.log 2>&1; echo "$f hi $(tail -1 logs/sub_${f}_hi_${n}.log)"
  TH=1 ./run_cfg.sh $m $t sub env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c
  TH=1 ./run_cfg.sh $m 60 sub60 env ENOMOTO_MIP_AS_SUBMIP=1 ENOMOTO_MIP_CUTOFF=$c
  TH=1 ./run_cfg.sh $m 60 solo env ENOMOTO_MIP_CUTOFF=$c
done <<'L'
phys_e2_3 50625.99 2.0
phys_e2_0 50625.99 2.15
neos-960392_0 -3.0 15.28
mzzv42z_1 -1192.0 16.65
mzzv42z_4 -20471.98 2.39
roll3000_6 13951.99 3.68
roll3000_2 1e30 20.0
beasleyC3_6 783.0 3.79
beasleyC3_3 937.0 4.06
L
