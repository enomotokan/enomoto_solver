#!/bin/bash
# 副作用: 40 問の残り (batch4 と同じ順)、4 スレッド・60 秒、mink = PROOF_KEEP_INT + PROOF_CMIR + PROOF_CMIR_POOL=nocap
W=analysis/tree_replay_20261022
MINK="ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap"
for P in gen-ip002 gen-ip054 gr4x6 markshare_4_0 mas74 neos5 p0201 flugpl dcmulti gt2 khb05250 qnet1 air03 blend2 fiber noswot neos-911970 30n20b8 binkar10_1 nw04 assign1-5-8 beavma blp-ar98 ran14x18-disj-8 h80x6320d supportcase26 timtab1 bppc4-08 cvs16r128-89 mik-250-20-75-4 neos-1456979 graph20-20-1rand; do
  [ -f $W/logs/side/${P}_mink.out ] || $W/run_any.sh $P $W/logs/side/${P}_mink 60 4 $MINK
done
