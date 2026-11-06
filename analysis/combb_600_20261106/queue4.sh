#!/bin/bash
# 得がどの部品から来るか (根の LP の時刻だけ見るので 60 秒): momentum1・satellites2-40・neos-873061 を尺度の調整だけ / HARRIS2=r だけ / NOISE_RESET だけ
cd "$(dirname "$0")/../.."
for p in neos-873061 momentum1 satellites2-40; do for c in combb_scaleonly combb_h2only combb_nronly; do RUN_CFG_TL=60 python analysis/combb_600_20261106/run.py $c $p 0; done; done
