#!/bin/bash
# 段階 1: 軽い 4 問 × 変種 7 つ × 1 回 (診断つき)
cd "$(dirname "$0")"
for p in 10teams noswot beasleyC3 neos-911970; do
  ./run_cfg.sh $p 60 b_cold ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_NO_SNAP_BASIS=1
  ./run_cfg.sh $p 60 b_snap ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_SNAP_BASIS_ALWAYS=1
  for v in rand,lex frac,lex far,lex rand,lex,dual dual; do
    ./run_cfg.sh $p 60 b_$v ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_DIVERSE_VERTEX=$v
  done
done
