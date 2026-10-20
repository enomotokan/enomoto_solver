until grep -q RESULT tf25_2.out 2>/dev/null; do sleep 20; done
./exp.sh diag_nc 300 4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_NODE_CUTS=1
./exp.sh diag_def 300 4 ENOMOTO_MIP_LOG=1
