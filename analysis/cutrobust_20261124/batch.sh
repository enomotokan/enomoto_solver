#!/bin/bash
# 使い方: bash batch.sh 名札 問題 [ENV=値 ...]  出発の頂点 0〜8 (ENOMOTO_MIP_MPSEP_START) を順に回す。同点の時間の揺れを除くため TIE_FEWER=1
tag=$1; p=$2; shift 2
for s in 0 1 2 3 4 5 6 7 8; do python3 one.py $p ${tag}_start$s ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER=1 ENOMOTO_MIP_MPSEP_START=$s "$@" | python3 -c "import json,sys;r=json.loads(sys.stdin.read());print(r['tag'],len(r['rounds']),r['rounds'][-1:],r.get('result',['?'])[3:])"; done
