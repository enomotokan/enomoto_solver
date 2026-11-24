#!/bin/bash
# 6 問 x {既定, SB_ADAPT} x {打ち切り値なし, 最適値の打ち切り値つき} を 600 秒で 1 本ずつ (終わった組は res.jsonl を見て飛ばす)。
# リポジトリの根から: bash analysis/sbadapt_20261124/all.sh
cd "$(dirname "$0")/../.."
D=analysis/sbadapt_20261124
declare -A CUT=( [satellites2-40]=-18.999981 [satellites2-60-fs]=-18.999981 [30n20b8]=302.000302 [dano3_5]=576.92549 [mcsched]=211913.211913 [swath1]=379.0716748 )
done_() { grep -q "\"tag\": \"$1\", \"problem\": \"$2\", \"tl\": 600" $D/res.jsonl 2>/dev/null; }
for p in satellites2-40 satellites2-60-fs 30n20b8 dano3_5 mcsched swath1; do
  for v in "b600:X=1" "ad600:ENOMOTO_MIP_SB_ADAPT=1" "bcut:ENOMOTO_MIP_CUTOFF=${CUT[$p]}" "adcut:ENOMOTO_MIP_SB_ADAPT=1 ENOMOTO_MIP_CUTOFF=${CUT[$p]}"; do
    tag=${v%%:*}; done_ $tag $p && continue
    bash $D/run.sh $tag $p 600 ${v#*:}
  done
done
