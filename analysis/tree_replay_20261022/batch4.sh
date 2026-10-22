#!/bin/bash
# 副作用: 40 問 (scripts/run_miplib_benchmark.py)、4 スレッド・60 秒、既定と試作 (kpnf) を 1 回ずつ交互に
W=analysis/tree_replay_20261022
ALL="ENOMOTO_MIP_LP_INT_BOUNDS=1 ENOMOTO_MIP_PROOF_KEEP_INT=1 ENOMOTO_MIP_PROOF_CMIR=1 ENOMOTO_MIP_PROOF_CMIR_POOL=nocap ENOMOTO_MIP_CUTPOOL_PROP=1"
mkdir -p $W/logs/side
for k in 1 2 3; do $W/run_any.sh neos-2978193-inde $W/logs/t4_kpnf_$k 60 4 $ALL; done
for P in gen-ip002 gen-ip054 pk1 gr4x6 markshare_4_0 mas74 mas76 neos5 p0201 misc07 flugpl dcmulti gt2 khb05250 qnet1 rout 10teams air03 blend2 fiber noswot qiu neos-911970 30n20b8 binkar10_1 neos-860300 eil33-2 nw04 assign1-5-8 beavma blp-ar98 ran14x18-disj-8 h80x6320d supportcase26 timtab1 bppc4-08 cvs16r128-89 mik-250-20-75-4 neos-1456979 graph20-20-1rand; do
  $W/run_any.sh $P $W/logs/side/${P}_base 60 4
  $W/run_any.sh $P $W/logs/side/${P}_kpnf 60 4 $ALL
done
