#!/bin/bash
# ns1116954 を FJ の種を変えて回す (FJ_FEAS_EXTEND + ROOT_GAP_EXIT)
cd /home/user/enomoto_solver/work_ucc; N=$PWD/../.miplib_cache/ns1116954.mps
for s in 1 2 3 4 5; do
  ./batch.sh e_ns_fjx_gx_seed$s $N 60 ENOMOTO_MIP_FJ_FEAS_EXTEND=1 ENOMOTO_MIP_ROOT_GAP_EXIT=1 ENOMOTO_MIP_FJ_DEBUG=1 ENOMOTO_T_MIP_FJ_SEED=$s | cut -c1-40,130-330
  grep "FJ2 (feas" logs/e_ns_fjx_gx_seed$s.log
done
