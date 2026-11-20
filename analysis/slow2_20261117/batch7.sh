#!/bin/bash
# unitcal_7: カットの選び方を緩める (MIN_REL 0.1・1 ラウンド 200 本)、種 1・2、600 秒。mzzv42z 種 1 で害を見る
D=$(cd "$(dirname "$0")" && pwd)
S="ENOMOTO_T_CUTSEL_MIN_REL=0.1 ENOMOTO_T_MIP_ROOT_MAX_CUTS=200"
"$D/one.sh" unitcal_7 ours 1 600 sel $S
"$D/one.sh" unitcal_7 ours 2 600 sel $S
"$D/one.sh" unitcal_7 ours 2 600 def
"$D/one.sh" mzzv42z ours 1 600 sel $S
