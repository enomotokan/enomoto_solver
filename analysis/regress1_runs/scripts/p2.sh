#!/bin/bash
cd /home/user/wt; L=$PWD/logs; export ENOMOTO_MIP_LOG=1
for r in 1 2; do
  for m in 1 nopump; do ENOMOTO_MIP_SYM_LEX_CANON=1 ENOMOTO_MIP_FJ_POOR_NOINC=$m scripts/run.sh proto1 neos-950242 60 $L/p2_lexpoor${m}_neos-950242_r$r.log; done
  for p in comp07-2idx comp21-2idx net12 nexp-150-20-8-5 ns1830653; do
    scripts/run.sh proto1 $p 60 $L/p2_none_${p}_r$r.log
    for m in 1 nopump; do ENOMOTO_MIP_FJ_POOR_NOINC=$m scripts/run.sh proto1 $p 60 $L/p2_poor${m}_${p}_r$r.log; done
  done
  scripts/run.sh proto1 neos-3381206-awhea 60 $L/p2_none_neos-3381206-awhea_r$r.log
  ENOMOTO_MIP_SYM_LEX_CANON=1 scripts/run.sh proto1 neos-3381206-awhea 60 $L/p2_lex_neos-3381206-awhea_r$r.log
done
echo done
