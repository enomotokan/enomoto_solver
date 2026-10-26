#!/bin/bash
# 証明の取り込みを増やす (作ったときに知らせる) + ほかの問題でも刈り込みを LP で確かめる
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
V="ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
./loop.sh share4 mc11 6 4 60 $D ENOMOTO_MIP_TREE_SHARE_PROOFS_AT_ONCE=1
for p in beasleyC3 rout neos-911970; do ./loop.sh pv4 $p 3 4 60 $V; done
