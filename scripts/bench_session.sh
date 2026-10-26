#!/bin/bash
# クラウドセッション 1 つで、MIPLIB 40 問 + 16 問 (4 スレッド・60 秒) を 1 つの設定で 1 回測る (scripts/bench_cand_20261020.py)。
# 使い方: scripts/bench_session.sh <名前> <回> <ビルドする commit> <push するブランチ> [ENV=VAL ...]
# 結果は benchmarks/{miplib_results,highsonly16}_4t_60s_20261020_<commit 7 桁>_<名前>_r<回>.{json,md} と
# benchmarks/cand_logs/<名前>_r<回>.{40,16}.log。10 問ごとと最後に commit して <ブランチ> に push する。
# 計測中はこのコンテナで他の重い処理をしないこと。
set -u
NAME=$1; RUN=$2; COMMIT=$3; BRANCH=$4; shift 4
REPO=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO"
VENV=$HOME/benchvenv
[ -x "$VENV/bin/python" ] || python3 -m venv "$VENV"
"$VENV/bin/pip" install -q maturin highspy numpy
"$VENV/bin/python" - <<'PY'
import sys; sys.path.insert(0, 'scripts')
import run_miplib_benchmark as r
from pathlib import Path
P16 = ("nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 "
       "uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1").split()
for n in list(r.PROBLEMS) + P16:
    r.fetch(n, Path('.miplib_cache'))
PY
# 計測するコードのビルド (作業ツリーとは別の worktree で)
WT=/tmp/benchwt
[ -d $WT ] || git worktree add --detach $WT "$COMMIT"
(cd $WT && git checkout -q --detach "$COMMIT" && rm -rf dist && "$VENV/bin/maturin" build --release -o dist 2>&1 | tail -1 && "$VENV/bin/pip" install -q --no-deps --force-reinstall dist/*.whl)
SHA=$(git rev-parse --short=7 "$COMMIT")
export ENOMOTO_BENCH_SHA=$SHA
push() {
  git add benchmarks/cand_logs/"${NAME}"_r"${RUN}".* benchmarks/*_"${SHA}"_"${NAME}"_r"${RUN}".* 2>/dev/null
  git commit -qm "計測 ($NAME r$RUN、$SHA): $1" || return 0
  for d in 2 4 8 16; do git push -q origin "HEAD:$BRANCH" && return 0; sleep $d; done
}
cd "$REPO" && "$VENV/bin/python" scripts/bench_cand_20261020.py "$NAME" "$RUN" "$@" | while read -r line; do
  echo "$line"
  n=$(cat benchmarks/cand_logs/"${NAME}"_r"${RUN}".*.log 2>/dev/null | wc -l)
  [ $((n % 10)) -eq 0 ] && push "途中 ($n 問)"
done
push "完了"
echo BENCH_SESSION_DONE
