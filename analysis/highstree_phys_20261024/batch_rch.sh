#!/bin/bash
# batch_rch.sh: ENOMOTO_MIP_ROOT_CUT_HIGHS (rch) の確かめ (60 秒・4 スレッド)。physiciansched6-2 を 4 回、ほかの 8 問を既定・rch で 2 回ずつ
cd "$(dirname "$0")"
for i in 1 2 3 4; do ./run_cfg.sh physiciansched6-2 60 rch ENOMOTO_MIP_ROOT_CUT_HIGHS=1; done
for p in mzzv42z timtab1 neos-960392 app1-1 qiu h80x6320d nw04 10teams; do
  for i in 1 2; do
    ./run_cfg.sh $p 60 def
    ./run_cfg.sh $p 60 rch ENOMOTO_MIP_ROOT_CUT_HIGHS=1
  done
done
