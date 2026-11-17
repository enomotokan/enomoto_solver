#!/bin/bash
# 課題 A の特徴集め: sw (k=0.5) が全種で速かった・遅かった問題を、既定と sw で 120 秒 (P は 60 秒) ログつきで回す (種 1、peg は種 2)
D=$(cd "$(dirname "$0")" && pwd)
run() { "$D/one.sh" $1 ours $2 $3 def; "$D/one.sh" $1 ours $2 $3 sw ENOMOTO_T_MIP_SUBWAIT=0.5; }
run 10teams 1 60
run 10teams 3 60
for p in neos-3627168-kasai supportcase33 pg5_34 CMS750_4 neos-662469 neos-2978193-inde; do run $p 1 120; done
run peg-solitaire-a3 2 120
