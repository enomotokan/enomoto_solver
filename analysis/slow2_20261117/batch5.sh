#!/bin/bash
# mzzv42z: wst (PRR_PRECHECK + HNFPRE のかけ直しは 10 列以上) を種 1・2、CLQPRB=1dgr との組を種 2、既定の種 2
D=$(cd "$(dirname "$0")" && pwd)
W="ENOMOTO_MIP_PRR_PRECHECK=1 ENOMOTO_T_MIP_HNFPRE_REPRE_MIN=10"
"$D/one.sh" mzzv42z ours 1 600 wst $W
"$D/one.sh" mzzv42z ours 2 600 wst $W
"$D/one.sh" mzzv42z ours 2 600 def
"$D/one.sh" mzzv42z ours 2 600 wstclqg $W ENOMOTO_MIP_CLQPRB=1dgr
"$D/one.sh" unitcal_7 ours 1 600 wst $W
