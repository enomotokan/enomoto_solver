#!/bin/sh
# 固定した作業木 (e2b2f0677) から全版を作り直す (2 回目)。(a) の ff は /home/user/bo_wt_ff (apply_ffloor.py をかけた作業木)。
# var/lto_v2 は 1 回目 (作業木でない) の残りなので使わない。
cd "$(dirname "$0")"
for n in base v2 v3 lto lto_v3; do sh build_var.sh $n; done
SRC=/home/user/bo_wt_ff OUT=ff sh build_var.sh base
SRC=/home/user/bo_wt_ff OUT=ff_lto sh build_var.sh lto
sh build_var.sh pgogen && sh pgo_train.sh && sh build_var.sh pgo && sh build_var.sh pgo_v3
