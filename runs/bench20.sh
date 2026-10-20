#!/bin/bash
# 判定に当たる 20 問 (固定費用型 18 問 + beavma・ran14x18-disj-8)、4 スレッド・60 秒。既定と SEP2 を 2 回ずつ
cd /home/user/enomoto_solver; B=runs/bench20
P="aflow30a aflow40b beasleyC1 beasleyC2 beasleyC3 g200x740 gr4x6 h50x2450 h80x6320 h80x6320d k16x240b mc11 mc7 mc8 p200x1188c r50x360 rout sp150x300d beavma ran14x18-disj-8"
S="python scripts/run_miplib_benchmark.py --threads 4 --time-limit 60 --only $P"
$S --out $B/def1.json > $B/def1.out 2>&1
for t in s2_1 def2 s2_2; do cp $B/def1.json $B/$t.json; done
ENOMOTO_MIP_FC_NODE_SEP2=1 $S --rerun-ours --out $B/s2_1.json > $B/s2_1.out 2>&1
$S --rerun-ours --out $B/def2.json > $B/def2.out 2>&1
ENOMOTO_MIP_FC_NODE_SEP2=1 $S --rerun-ours --out $B/s2_2.json > $B/s2_2.out 2>&1
echo DONE > $B/done
