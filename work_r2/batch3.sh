until grep -c HRESULT highs1.log 2>/dev/null | grep -q 4; do sleep 20; done
for rep in 1 2; do
./exp.sh nodecuts_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_NODE_CUTS=1
./exp.sh nca_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_NODE_CUT_ADAPT=1
./exp.sh fcsep2_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_FC_NODE_SEP2=1
done
