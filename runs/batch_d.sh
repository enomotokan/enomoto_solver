#!/bin/bash
R=/home/user/enomoto_solver/runs
for i in 1 2; do
$R/go.sh s2k$i beasleyC3 60 ENOMOTO_MIP_FC_NODE_SEP2=1
$R/go.sh s2k$i mc11 60 ENOMOTO_MIP_FC_NODE_SEP2=1
done
$R/go.sh cut_def beasleyC3 60 ENOMOTO_MIP_CUTOFF=754.5
$R/go.sh cut_s2k beasleyC3 60 ENOMOTO_MIP_CUTOFF=754.5 ENOMOTO_MIP_FC_NODE_SEP2=1
python $R/run_highs.py beasleyC3 60 754.5 > $R/logs/cut_highs4_beasleyC3.log 2>&1
$R/go.sh cut_def mc11 60 ENOMOTO_MIP_CUTOFF=11689.5
$R/go.sh cut_s2k mc11 60 ENOMOTO_MIP_CUTOFF=11689.5 ENOMOTO_MIP_FC_NODE_SEP2=1
python $R/run_highs.py mc11 60 11689.5 > $R/logs/cut_highs4_mc11.log 2>&1
echo DONE
