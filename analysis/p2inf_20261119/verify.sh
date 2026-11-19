#!/bin/bash
# P2-INF: ENOMOTO_MIP_LPINF_ROWTINY=1 の正しさの確かめ (親の依頼)。リポジトリの根から: PYTHONPATH=<試作のビルド> bash analysis/p2inf_20261119/verify.sh
# (1) neos-4722843-widden (誤答の元) を 1・4 スレッド × 種 1〜3、デバッグ解つき (最適 25009.662227、900 秒)。
# (2) release019 / wrong_widden と同じ fuzz 7 本と、LP の実行不能の射線を使う衝突解析の fuzz (fuzz_cutconf、既定の設定で)。ログは fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
D=analysis/p2inf_20261119; L=$D/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_LPINF_ROWTINY=1
SOL=$(mktemp -d)/widden_pre_s1.sol; gunzip -c analysis/wrong_widden_20261117/widden_pre_s1.sol.gz > $SOL
for th in 4 1; do python3 $D/wdn.py neos-4722843-widden wdn_t$th 1-3 25009.662227 $th ENOMOTO_MIP_DEBUG_SOL=$SOL ENOMOTO_MIP_LPINF_ROWTINY=1; done
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run fuzz_cutconf python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 0
