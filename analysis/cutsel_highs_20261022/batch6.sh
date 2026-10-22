#!/bin/bash
# batch5 の残り (コンテナの再起動で止まった分) と、EQUIV2_HSEL (代入した求解だけ) の確かめ
cd "$(dirname "$0")"
./run_cfg.sh neos5 60 hs_age0 ENOMOTO_MIP_ROOT_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
./run_cfg.sh timtab1 60 def
./run_cfg.sh timtab1 60 hs ENOMOTO_MIP_ROOT_HSEL=1
./run_cfg.sh timtab1 60 hs_age0 ENOMOTO_MIP_ROOT_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
for p in mzzv42z roll3000 neos-911970 beasleyC3 mc11 10teams h80x6320d misc07 qnet1 neos5 timtab1; do
  ./run_cfg.sh $p 60 def
  ./run_cfg.sh $p 60 hs ENOMOTO_MIP_ROOT_HSEL=1
  ./run_cfg.sh $p 60 hs_age0 ENOMOTO_MIP_ROOT_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
done
E2ALL="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_EQUIV2_FJ=1 ENOMOTO_MIP_EQUIV2_TAB=1"
for i in 1 2 3 4; do
  ./run_cfg.sh physiciansched6-2 60 e2eh0 ENOMOTO_MIP_DEBUG_SEP=1 $E2ALL ENOMOTO_MIP_EQUIV2_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
done
for p in beasleyC3 neos-911970; do
  ./run_cfg.sh $p 60 e2eh0 $E2ALL ENOMOTO_MIP_EQUIV2_HSEL=1 ENOMOTO_T_MIP_HSEL_AGE=0
done
