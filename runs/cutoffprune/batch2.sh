#!/bin/bash
# 11728 の型の再現 (このブランチ、設定 noF の環境変数はこのブランチには存在しないので既定と同じ)
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1"
./loop.sh def4 mc11 5 4 60 $D
./loop.sh c11728_4 mc11 5 4 60 $D ENOMOTO_MIP_CUTOFF=11728
