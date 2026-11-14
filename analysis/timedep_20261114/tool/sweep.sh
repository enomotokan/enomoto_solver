#!/bin/bash
# 文字ごとの値の掃引 (基準 T = mplwnNebtyc、54 問、種 1、手間の上限 wl_T.json)。結果は tool/sweep.jsonl (札 = 文字_値)
cd "$(dirname "$0")/.."
S=10teams,30n20b8,air03,assign1-5-8,beavma,binkar10_1,blend2,blp-ar98,bppc4-08,cvs16r128-89,dcmulti,drayage-100-23,drayage-25-23,eil33-2,exp-1-500-5-5,fiber,flugpl,gen-ip002,gen-ip054,gr4x6,gt2,h80x6320d,hypothyroid-k1,khb05250,markshare_4_0,mas74,mas76,mik-250-20-75-4,misc07,mzzv42z,neos-1456979,neos-3083819-nubu,neos-827175,neos-860300,neos-911970,neos-960392,neos17,neos5,neos8,noswot,ns1116954,nu25-pr12,nw04,p0201,p200x1188c,physiciansched6-2,pk1,qiu,qnet1,ran14x18-disj-8,roll3000,rout,sp150x300d,swath1
T=ENOMOTO_MIP_TIMEDEP=mplwnNebtyc
run() { python tool.py tool/sweep.jsonl $1 $S 1 --wl tool/wl_T.json $T $2; }
run w6 ENOMOTO_T_MIP_CUT_WORK_CAP=6; run w24 ENOMOTO_T_MIP_CUT_WORK_CAP=24; run w48 ENOMOTO_T_MIP_CUT_WORK_CAP=48
run n2.5 ENOMOTO_T_MIP_TIMEDEP_LNS_MIN=2.5; run n10 ENOMOTO_T_MIP_TIMEDEP_LNS_MIN=10; run n20 ENOMOTO_T_MIP_TIMEDEP_LNS_MIN=20
run t0.05 ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC=0.05; run t0.2 ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC=0.2; run t0.4 ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC=0.4
run e0.5 ENOMOTO_T_MIP_TREE_RESTART_GUARD_ROOT=0.5; run e2 ENOMOTO_T_MIP_TREE_RESTART_GUARD_ROOT=2; run e4 ENOMOTO_T_MIP_TREE_RESTART_GUARD_ROOT=4
