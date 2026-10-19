cd /home/user/wt/neos-911970/runs
/tmp/with_slot.sh python run_highs.py 60 > h4_60.log 2>&1
./cfg.sh t4_def 60
./cfg.sh t4_hcm 60 ENOMOTO_MIP_HIGHS_CUTMGMT=1
./cfg.sh t4_hselhcm 60 ENOMOTO_MIP_HIGHS_CUTMGMT=1 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_T_MIP_CUT_ROUNDS=100
./cfg.sh t4_hsel 60 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_T_MIP_CUT_ROUNDS=100
echo DONE > batch1.done
