#!/bin/bash
# 待ち行列 (改良 11-08): 最新のブランチ (ROOTLP_SCALE が既定) で作り直した wheel。avoca 300 秒 → rococoB10・C10 600 秒 → flugpl 60 秒 (種 1・2)
cd "$(dirname "$0")"
python run.py v2_avoca300.jsonl 300 def,hcut_b,hcut_bc,hcut_bcp,hcut_abcp 1,2 neos-3373491-avoca > v2.log 2>&1
python run.py v2_rococo600.jsonl 600 def,hcut_b,hcut_bc,hcut_bcp,hcut_abcp 1,2 rococoB10-011000 >> v2.log 2>&1
python run.py v2_rococo600.jsonl 600 def,hcut_b,clc,hcut_b_clc,hcut_bc,hcut_bc_clc 1,2 rococoC10-001000 >> v2.log 2>&1
python run.py v2_flugpl.jsonl 60 def,hcut_b,hcut_bc,hcut_bcp,hcut_abcp 1,2 flugpl >> v2.log 2>&1
echo done > chain5_done
