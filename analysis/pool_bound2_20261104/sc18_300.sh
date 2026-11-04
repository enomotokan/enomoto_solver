# supportcase18 の 300 秒 (診断つき) を設定ごとに回す (pool_bound2、2026-11-04)
cd "$(dirname "$0")"
export ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_POOL2_DIAG=1
ENOMOTO_T_MIP_TREE_BALANCE_Q=0 python run.py diag_q0 supportcase18 300
ENOMOTO_T_MIP_TREE_BALANCE_Q=0 ENOMOTO_T_MIP_TREE_BALANCE_EVERY=4 python run.py diag_q0e4 supportcase18 300
ENOMOTO_MIP_TREE_SHARED_QUEUE=1 python run.py diag_shared supportcase18 300
ENOMOTO_MIP_TREE_KEEP_SIBLINGS=1 python run.py diag_keepsib supportcase18 300
ENOMOTO_MIP_POOL2_TAKE=h python run.py diag_p2h supportcase18 300
ENOMOTO_MIP_POOL2_TAKE=g,h,k4 python run.py diag_p2ghk4 supportcase18 300
ENOMOTO_MIP_POOL2_TAKE=g,i,k4 python run.py diag_p2gik4 supportcase18 300
