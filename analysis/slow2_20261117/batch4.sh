#!/bin/bash
# mzzv42z: 空回りを除く試作と CLQPRB (種 1、600 秒)
D=$(cd "$(dirname "$0")" && pwd)
W="ENOMOTO_MIP_PRR_PRECHECK=1 ENOMOTO_T_MIP_HNFPRE_REPRE_MIN=1000000"
"$D/one.sh" mzzv42z ours 1 600 waste $W
"$D/one.sh" mzzv42z ours 1 600 clq ENOMOTO_MIP_CLQPRB=1
"$D/one.sh" mzzv42z ours 1 600 wasteclq $W ENOMOTO_MIP_CLQPRB=1
