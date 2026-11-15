#!/bin/bash
# HiGHS のアブレーション: 問題 秒 を引数に、設定を順に回す
p=$1; t=$2; shift 2
cfgs=${CFGS:-"def nosb rel1 norestart noheur nosym nonodecut"}
for c in $cfgs; do
  case $c in
    def) o="";;
    nosb) o="mip_pscost_minreliable=0";;
    rel1) o="mip_pscost_minreliable=1";;
    rel4) o="mip_pscost_minreliable=4";;
    norestart) o="mip_allow_restart=false";;
    noheur) o="mip_heuristic_effort=0 mip_heuristic_run_feasibility_jump=false mip_heuristic_run_rins=false mip_heuristic_run_rens=false mip_heuristic_run_root_reduced_cost=false";;
    nosym) o="mip_detect_symmetry=false";;
    nonodecut) o="mip_allow_cut_separation_at_nodes=false";;
    age3) o="mip_lp_age_limit=3";;
    *) o="";;
  esac
  grep -q "\"prob\": \"$p\", \"tag\": \"$c\"" hres.jsonl 2>/dev/null && continue
  timeout $((t+120)) python hrun.py $p $t $c $o
done
