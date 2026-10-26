#!/bin/bash
# batch_rch2.sh: 選択を緩めた ENOMOTO_MIP_ROOT_CUT_HIGHS (rch2: 1 ラウンド 500 本・平行度 0.9) の確かめ (60 秒・4 スレッド)。
# physiciansched6-2 を 4 回 (rch2) と ENOMOTO_MIP_IMPLINT_TU 付き 2 回、ほかの 8 問を rch2 で 2 回ずつ (既定は batch_rch.sh の def を使う)
cd "$(dirname "$0")"
for i in 1 2 3 4; do ./run_cfg.sh physiciansched6-2 60 rch2 ENOMOTO_MIP_ROOT_CUT_HIGHS=1; done
for i in 1 2; do ./run_cfg.sh physiciansched6-2 60 rch2itu ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_MIP_IMPLINT_TU=1; done
for p in mzzv42z timtab1 neos-960392 app1-1 qiu h80x6320d nw04 10teams; do
  for i in 1 2; do ./run_cfg.sh $p 60 rch2 ENOMOTO_MIP_ROOT_CUT_HIGHS=1; done
done
