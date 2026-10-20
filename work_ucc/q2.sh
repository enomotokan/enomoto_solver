#!/bin/bash
cd /home/user/enomoto_solver/work_ucc; U=$PWD/../.miplib_cache/uccase12.mps
./batch.sh e_ucc_bgb_4 $U 60 ENOMOTO_MIP_BG_CUTS_BASIS=1 | cut -c1-60,150-400
./batch.sh e_ucc_bgb_5 $U 60 ENOMOTO_MIP_BG_CUTS_BASIS=1 | cut -c1-60,150-400
./batch.sh e_ucc_all_1 $U 60 ENOMOTO_MIP_BG_CUTS_BASIS=1 ENOMOTO_MIP_ROOT_GAP_EXIT=1 ENOMOTO_MIP_FJ_FEAS_EXTEND=1 | cut -c1-60,150-400
./batch.sh e_ucc_bgb_gap4_1 $U 60 ENOMOTO_MIP_BG_CUTS_BASIS=1 ENOMOTO_MIP_REL_GAP=1e-4 | cut -c1-60,150-400
./batch.sh e_ucc_bgb_gap4_2 $U 60 ENOMOTO_MIP_BG_CUTS_BASIS=1 ENOMOTO_MIP_REL_GAP=1e-4 | cut -c1-60,150-400
./batch.sh e_ucc_def_2 $U 60 | cut -c1-60,150-400
python run_highs.py $U 120 mip_heuristic_run_rins=false mip_heuristic_run_rens=false > logs/h_ucc_norinsrens.log 2>&1; echo "h_ucc_norinsrens $(grep RESULT logs/h_ucc_norinsrens.log)"
python run_highs.py $U 120 mip_rel_gap=0 > logs/h_ucc_gap0.log 2>&1; echo "h_ucc_gap0 $(grep RESULT logs/h_ucc_gap0.log)"
