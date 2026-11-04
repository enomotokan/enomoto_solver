for p in rococoC10-001000 neos-1445765; do
./rootrun.sh $p 20 def
./rootrun.sh $p 20 cselh ENOMOTO_MIP_CUTSEL_HIGHS=1
./rootrun.sh $p 20 hcm ENOMOTO_MIP_HIGHS_CUTMGMT=1
./rootrun.sh $p 20 ph2 ENOMOTO_MIP_CUT_PHASE2=1
./rootrun.sh $p 20 stallh ENOMOTO_MIP_CUT_STALL_HIGHS=1 ENOMOTO_T_MIP_CUT_ROUNDS=100
./rootrun.sh $p 20 cliqon ENOMOTO_MIP_CLIQUE_CUTS=on
./rootrun.sh $p 20 lcov ENOMOTO_MIP_LIFTED_COVER=1
done
