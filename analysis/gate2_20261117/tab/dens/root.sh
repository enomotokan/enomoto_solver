#!/bin/bash
# 密度で割る点数の試作 (README 14 節) の根だけの試し: 9 問 × 設定を feat.py (根だけ・60 秒・4 スレッド・種 1・DEBUG_SEP) で解く。
# 使い方: bash root.sh <PYTHONPATH (試作のビルド)>。結果は ../feat.jsonl の札 d_<設定>、ログは ../logs/feat_<問題>_d_<設定>.log.gz
cd "$(dirname "$0")/.."
PR=qnet1,10teams,neos-911970,mc11,nw04,30n20b8,roll3000,neos-3627168-kasai,fiball
run() { tag=$1; shift; FEAT_PROBS=$PR PYTHONPATH=$PY nice -n 10 python3 feat.py d_$tag "$@"; }
PY=$1
run def
run tab ENOMOTO_MIP_TAB_HIGHS_RULES=1
for k in a b c; do for al in 0.5 1; do run ${k}${al} ENOMOTO_MIP_TAB_DENS=$k ENOMOTO_T_MIP_TAB_DENS_ALPHA=$al; done; done
echo ROOT_DONE
