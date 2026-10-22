#!/bin/bash
# 段階 2: 根の LP が重い 3 問。既定 (ゲート: 写しの基底あり)・基底なし・rand,lex,dual を 60 秒で 2 回ずつ。
# uccase12・physiciansched6-2 は 60 秒では木に入らないことが多いので、再スタートなし・120 秒の診断も 1 回ずつ。
cd "$(dirname "$0")"
for k in 1 2; do
  for p in mzzv42z uccase12 physiciansched6-2; do
    ./run_cfg.sh $p 60 h_def ENOMOTO_MIP_TREE_DIAG=1
    ./run_cfg.sh $p 60 h_cold ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_NO_SNAP_BASIS=1
    ./run_cfg.sh $p 60 h_rand,lex,dual ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_TREE_DIVERSE_VERTEX=rand,lex,dual
  done
done
for p in uccase12 physiciansched6-2; do
  ./run_cfg.sh $p 120 h120nr_rand,lex,dual ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_NO_RESTART=1 ENOMOTO_MIP_TREE_DIVERSE_VERTEX=rand,lex,dual
  ./run_cfg.sh $p 120 h120nr_cold ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_NO_RESTART=1 ENOMOTO_MIP_TREE_NO_SNAP_BASIS=1
done
