#!/bin/bash
# 根の LP を見る短い回 (20 秒): 既定・CLIQUE_EXTEND_FIX・それと PROBE_EQUIV_FIX
cd $(dirname $0)
./run_cfg.sh mzzv42z 20 r_def
./run_cfg.sh mzzv42z 20 r_xfix ENOMOTO_MIP_CLIQUE_EXTEND_FIX=1
./run_cfg.sh mzzv42z 20 r_xfixeq ENOMOTO_MIP_CLIQUE_EXTEND_FIX=1 ENOMOTO_MIP_PROBE_EQUIV_FIX=1
./run_cfg.sh mzzv42z 20 r_eq ENOMOTO_MIP_PROBE_EQUIV_FIX=1
