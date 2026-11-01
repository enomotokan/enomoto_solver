#!/bin/bash
# neos-960392 の根のサブ MIP を書き出す (/tmp/dump/<名>_<番号>.mps。0 番が根の RENS の近傍)。リポジトリの根から実行する。
# P1 (11702 列) は外側の根のカットを 3 ラウンドにして固定する (analysis/neos960392_20261101.md の 3 節、5/5 で P1)。
# P2 (11660 列) は既定 (この機械では大半が P2)。どちらの道になったかはログの "sub-MIP dump" の行の free integer columns で確かめる。
D=analysis/neos960392_nested_20261101
H=analysis/neos960392_20261101/highs
mkdir -p /tmp/dump
ENOMOTO_MIP_LOG=1 ENOMOTO_T_MIP_CUT_ROUNDS=3 ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS=25 ENOMOTO_MIP_SUBMIP_DUMP=/tmp/dump/p1_ python $H/run_ours.py 2> $D/logs/dump_p1.log
for i in 1 2 3; do
  ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_DUMP=/tmp/dump/p2${i}_ python $H/run_ours.py 2> $D/logs/dump_p2_$i.log
  grep -q "p2${i}_0.mps: rows .*free integer columns 11660" $D/logs/dump_p2_$i.log && break
done
grep "sub-MIP dump" $D/logs/dump_*.log
