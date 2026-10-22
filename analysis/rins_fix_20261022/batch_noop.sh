#!/bin/bash
# 条件つきにした後の HL: 暫定解が FJ だけでない問題では既定と同じ経路になることの確認と physiciansched6-2
cd "$(dirname "$0")"
./run_cfg.sh timtab1 60 hlc ENOMOTO_MIP_ROOT_SUBMIP_HL=1
./run_cfg.sh uccase12 60 hlc ENOMOTO_MIP_ROOT_SUBMIP_HL=1
./run_cfg.sh physiciansched6-2 60 def_hlc ENOMOTO_MIP_ROOT_SUBMIP_HL=1
