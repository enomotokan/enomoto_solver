#!/bin/bash
# クラウドセッション 1 つで、問題の組 (空白区切り) を設定ファイルの全設定で交互に測る (4 スレッド・60 秒、scripts/bench_cand_20261020.py)。
# 機械ごとの速さの違いを打ち消すため、1 問ずつ全設定を続けて解く (1 回目は設定ファイルの順、2 回目は逆順)。
# 使い方: scripts/bench_interleave.sh "<問題 ...>" <設定ファイル> <回数> <ビルドする commit> <push するブランチ> <接尾辞>
# 設定ファイルの各行: "名前 ENV=VAL ..."。結果は scripts/bench_session.sh と同じ形のファイル (名前<接尾辞>_r<回>。
# 接尾辞はセッションごとに変える (同じファイルを複数のセッションが書くと取り込むときにぶつかる)。
# 機械の情報は benchmarks/cand_logs/machine_<ブランチの最後の部分>.txt。問題ごとに commit して push する。
set -u
PROBS=$1; CFG=$2; RUNS=$3; COMMIT=$4; BRANCH=$5; SUF=$6
REPO=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO"
VENV=$HOME/benchvenv
[ -x "$VENV/bin/python" ] || python3 -m venv "$VENV"
"$VENV/bin/pip" install -q maturin highspy numpy
PROBS="$PROBS" "$VENV/bin/python" - <<'PY'
import os, sys; sys.path.insert(0, 'scripts')
import run_miplib_benchmark as r
from pathlib import Path
for n in os.environ['PROBS'].split():
    r.fetch(n, Path('.miplib_cache'))
PY
WT=/tmp/benchwt
[ -d $WT ] || git worktree add --detach $WT "$COMMIT"
(cd $WT && git checkout -q --detach "$COMMIT" && rm -rf dist && "$VENV/bin/maturin" build --release -o dist 2>&1 | tail -1 && "$VENV/bin/pip" install -q --no-deps --force-reinstall dist/*.whl)
SHA=$(git rev-parse --short=7 "$COMMIT")
export ENOMOTO_BENCH_SHA=$SHA
M=benchmarks/cand_logs/machine${SUF}.txt
{ echo "problems: $PROBS"; nproc; lscpu | grep -E "Model name|MHz|L3"; } > "$M"
mapfile -t LINES < <(grep -v '^\s*$' "$CFG")
for p in $PROBS; do
  for r in $(seq 1 "$RUNS"); do
    if [ $((r % 2)) -eq 1 ]; then order=$(seq 0 $((${#LINES[@]} - 1))); else order=$(seq $((${#LINES[@]} - 1)) -1 0); fi
    for k in $order; do
      read -r name envs <<< "${LINES[$k]}"
      ENOMOTO_BENCH_ONLY=$p "$VENV/bin/python" scripts/bench_cand_20261020.py "$name$SUF" "$r" $envs | grep -v "ALL DONE"
    done
  done
  git add "$M" benchmarks/cand_logs/ benchmarks/*_"${SHA}"_* 2>/dev/null
  git commit -qm "計測 (交互、$SHA): $p" || true
  for d in 2 4 8 16; do git push -q origin "HEAD:$BRANCH" && break; sleep $d; done
done
echo BENCH_SESSION_DONE
