#!/bin/bash
# WINWHY の切り分け: 主因と見た部品を環境変数で 1 つずつ切って、ENOMOTO だけを種 1・2 で 120 秒回す (既定の種 2 も)。
# 出力: res_abl_<タグ>.jsonl、logs/<問題>.<タグ>[_s2].log.gz。ソルバーは同時に 1 つ。
D=$(cd "$(dirname "$0")" && pwd)
ONE=ENOMOTO_T_MIP_TREE_SPAWN_NODES=1000000000   # 木の従のスレッドを始めない (木 1 本。FJ・サブ MIP などのスレッドはそのまま)
run() { p=$1; tag=$2; env=$3; for s in 1 2; do python3 $D/run.py abl $p --only ours --tag $tag --env "$env" --seed $s; done; }
for p in decomp2 app1-1 binkar10_1 neos-950242 fhnw-binpack4-48 neos-3004026-krka markshare_4_0 pk1 mas76 neos5 rmatr100-p10 ns1208400 cbs-cta roll3000 n5-3 piperout-08; do
  python3 $D/run.py abl $p --only ours --tag def --seed 2
done
run decomp2 fold0 ENOMOTO_MIP_FOLD=0
run decomp2 nolmix ENOMOTO_MIP_NO_FJ_LMIP_MIX=1
run app1-1 nolpprobe ENOMOTO_MIP_NO_LP_PROBE=1
run neos-950242 noclqext ENOMOTO_MIP_NO_CLIQUE_EXTEND=1
run neos-950242 nolmix ENOMOTO_MIP_NO_FJ_LMIP_MIX=1
run fhnw-binpack4-48 fjonly ENOMOTO_MIP_FEAS_FJ_EARLY=1
run neos-3004026-krka fjonly ENOMOTO_MIP_FEAS_FJ_EARLY=1
for p in binkar10_1 markshare_4_0 pk1 mas76 neos5 rmatr100-p10 roll3000 n5-3; do run $p onetree $ONE; done
run ns1208400 foldp ENOMOTO_MIP_FOLD=rip
run cbs-cta foldp ENOMOTO_MIP_FOLD=rip
