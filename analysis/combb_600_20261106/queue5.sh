#!/bin/bash
# 既定化の候補 comb (尺度の調整 + HARRIS2=r、雑音の作り直しなし) を 600 秒で: 効いた問題と害の出た問題
cd "$(dirname "$0")/../.."
for p in momentum1 satellites2-40 supportcase42 neos-873061; do RUN_CFG_TL=600 python analysis/combb_600_20261106/run.py comb $p 0; done
