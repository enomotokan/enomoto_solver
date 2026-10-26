#!/bin/bash
# 部品の切り分け: C (E2 + CCE + CUTSEL_PROBE=rate) に ENOMOTO_MIP_CUTSEL_PROBE_ALT で POOL の部品を 1 つずつ外した形を physiciansched6-2 で 1 回ずつ
cd "$(dirname "$0")"
C="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1 ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3"
ALL=SNPGTLFW
./run_cfg.sh physiciansched6-2 60 altfull $C
./run_cfg.sh physiciansched6-2 60 keep $C ENOMOTO_MIP_CUTSEL_PROBE_KEEP=1
for x in S N P G T L F W; do ./run_cfg.sh physiciansched6-2 60 alt_no$x $C ENOMOTO_MIP_CUTSEL_PROBE_ALT=${ALL//$x/}; done
echo ALT DONE
