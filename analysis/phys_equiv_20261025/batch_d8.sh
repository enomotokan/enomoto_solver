#!/bin/bash
# 形 D の physiciansched6-2 を MIP_LOG つきで 8 回 (種 0 を 3 回、種 1〜5)
cd "$(dirname "$0")"
D="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1 ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3 ENOMOTO_MIP_CUTSEL_PROBE_ALT=NS"
for s in 0 0 0 1 2 3 4 5; do ./run_cfg.sh physiciansched6-2 60 dlog_s$s $D ENOMOTO_T_MIP_SEED=$s; done
echo D8 DONE
