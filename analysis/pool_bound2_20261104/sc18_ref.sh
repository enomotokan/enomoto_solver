# supportcase18 の 300 秒: 根の値 47.18666666666667 のノードを分けたとき LP の値も根の値のままかを数える (pool_bound2、2026-11-04)
cd "$(dirname "$0")"
export ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_POOL2_DIAG=1 ENOMOTO_T_MIP_POOL2_DIAG_REF=47.18666666666667
python run.py ref_def supportcase18 300
ENOMOTO_MIP_TREE_SHARED_QUEUE=1 ENOMOTO_T_MIP_BB_EVERY_INC=1 python run.py ref_shared_bb1 supportcase18 300
