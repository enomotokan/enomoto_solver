until grep -q RESULT nodecuts_4.out 2>/dev/null; do sleep 20; done
for rep in 3 4 5; do
./exp.sh fcsep2_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_FC_NODE_SEP2=1
./exp.sh def_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1
done
for rep in 1 2; do
./exp.sh fcsep_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_FC_NODE_SEP=1
./exp.sh fcsep2pool_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_FC_NODE_SEP2=1 ENOMOTO_MIP_RESTART_CUTS_TO_POOL=1
done
