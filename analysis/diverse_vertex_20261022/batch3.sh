#!/bin/bash
# 段階 3: 軽い 4 問 × (既定・dual・rand,lex,dual) × 4 回 (結果の比較。揺れが大きいので交互に回す)
cd "$(dirname "$0")"
for k in 1 2 3 4; do
  for p in 10teams noswot beasleyC3 neos-911970; do
    ./run_cfg.sh $p 60 c_def ENOMOTO_MIP_TREE_DIAG=1
    ./run_cfg.sh $p 60 c_dual ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_DIVERSE_VERTEX=dual
    ./run_cfg.sh $p 60 c_rld ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_DIVERSE_VERTEX=rand,lex,dual
  done
done
