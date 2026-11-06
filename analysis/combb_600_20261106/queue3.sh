#!/bin/bash
# queue2.sh の後: supportcase42 で HARRIS2=r と NOISE_RESET を分ける。診断の出力 (ENOMOTO_DEBUG_EXT_ITERS) で安全モード・雑音の作り直しが起きたかを見る
cd "$(dirname "$0")/../.."
while pgrep -f "combb_600_20261106/queue2.sh" >/dev/null; do sleep 20; done
for c in combb_h2only combb_nronly; do RUN_CFG_TL=600 python analysis/combb_600_20261106/run.py $c supportcase42 0; done
for c in def combb; do RUN_DEBUG_EXT=1 RUN_CFG_TL=40 python analysis/combb_600_20261106/run.py $c supportcase42 0; done
for c in def combb; do RUN_DEBUG_EXT=1 RUN_THREADS=1 RUN_CFG_TL=60 python analysis/combb_600_20261106/run.py $c timtab1 0; done
