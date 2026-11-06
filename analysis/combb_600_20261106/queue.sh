#!/bin/bash
# 順に流す (1 本ずつ、4 スレッド)。timtab1 60 秒 ×5 ×2 設定 → 12 問 600 秒 回 0 → 回 1
cd "$(dirname "$0")/../.."
for r in 0 1 2 3 4; do for c in def combb; do RUN_CFG_TL=60 python analysis/combb_600_20261106/run.py $c timtab1 $r; done; done
P="uccase12 supportcase42 satellites2-40 momentum1 uccase9 snp-02-004-104 sing44 sing326 thor50dday neos-873061 rail01 opm2-z10-s4"
for r in 0 1; do for p in $P; do for c in combb def; do RUN_CFG_TL=600 python analysis/combb_600_20261106/run.py $c $p $r; done; done; done
