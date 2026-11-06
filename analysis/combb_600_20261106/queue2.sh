#!/bin/bash
# queue.sh の後の切り分け: supportcase42 の RINS の遅さ (尺度の調整のどれが効くか)、timtab1 を 1 スレッドで (同じ道筋か)
cd "$(dirname "$0")/../.."
while pgrep -f "combb_600_20261106/queue.sh" >/dev/null; do sleep 30; done
for c in combb_m17k combb_noscale combb_scaleonly; do RUN_CFG_TL=600 python analysis/combb_600_20261106/run.py $c supportcase42 0; done
for c in def combb combb_noscale; do RUN_THREADS=1 RUN_CFG_TL=60 python analysis/combb_600_20261106/run.py $c timtab1 0; done
for c in def combb; do RUN_THREADS=1 RUN_CFG_TL=60 python analysis/combb_600_20261106/run.py $c timtab1 1; done
