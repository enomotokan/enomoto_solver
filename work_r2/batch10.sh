./exp.sh pool_3 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_RESTART_CUTS_TO_POOL=1
./exp.sh pool_4 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_RESTART_CUTS_TO_POOL=1
for p in neos-911970 10teams binkar10_1 mas76 misc07 mik-250-20-75-4 neos-860300 eil33-2 qiu rout timtab1 neos5; do
PROB=$p ./exp.sh side_${p}_def 60 4
PROB=$p ./exp.sh side_${p}_pool 60 4 ENOMOTO_MIP_RESTART_CUTS_TO_POOL=1
done
