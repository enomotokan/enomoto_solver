#!/bin/bash
# クラウドセッション 1 つでパラメータの計測を走らせる (MIPLIB 40 問、1 問 90 秒、threads=1、4 問同時)。
# 使い方: scripts/mip_bench/run_remote.sh <名前 (w1 など)> <設定ファイル> <push するブランチ> [ビルドする commit]
# 設定ファイルの各行: "タグ 環境変数=値 ..."。最初に基準 (環境変数なし) を 2 回測り、続けて各行を 1 回ずつ測る。
# 結果は scripts/mip_bench/results/<名前>/res_<タグ>.json に置き、1 回ごとに commit して push する。
# 計測中はこのコンテナで他の重い処理をしないこと (同時に動く処理で時間が 15〜35% 狂う)。
set -u
W=$1; CFG=$2; BRANCH=$3; COMMIT=${4:-be68636}
REPO=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$REPO/scripts/mip_bench/work; OUT=$REPO/scripts/mip_bench/results/$W
mkdir -p "$WORK/sol" "$OUT"; export MIP_BENCH_DIR=$WORK
VENV=$HOME/mbvenv; [ -x "$VENV/bin/python" ] || python3 -m venv "$VENV"; PY=$VENV/bin/python
"$PY" -m pip install -q highspy numpy
cd "$REPO"
"$PY" -c "import sys; sys.path.insert(0,'scripts'); import run_miplib_benchmark as r; from pathlib import Path; [r.fetch(n, Path('.miplib_cache')) for n in r.PROBLEMS]"
curl -sS -o "$WORK/miplib.solu" https://miplib.zib.de/downloads/miplib2017-v31.solu
[ -s "$WORK/ref.json" ] || "$PY" scripts/mip_bench/ref.py "$WORK" > "$WORK/ref.log" 2>&1
# 計測するコードのビルド (作業ツリーとは別の worktree で)
[ -d /tmp/mbwt ] || git worktree add --detach /tmp/mbwt "$COMMIT"
(cd /tmp/mbwt && git checkout -q --detach "$COMMIT" && CARGO_TARGET_DIR=/tmp/mbtarget cargo build --release --lib -q)
SUF=$("$PY" -c "import sysconfig; print(sysconfig.get_config_var('EXT_SUFFIX'))")
mkdir -p "$WORK/pkg/enomoto_solver"; cp /tmp/mbwt/python/enomoto_solver/*.py "$WORK/pkg/enomoto_solver/"
cp /tmp/mbtarget/release/lib_core.so "$WORK/pkg/enomoto_solver/_core$SUF"
run() {
  local tag=$1; shift
  (cd "$WORK" && "$PY" "$REPO/scripts/mip_bench/bench.py" --out "res_$tag.json" -j 4 --env "PYTHONPATH=$WORK/pkg" "$@" > "res_$tag.log" 2>&1)
  cp "$WORK/res_$tag.json" "$WORK/res_$tag.log" "$OUT/"
  git add "$OUT" && git commit -qm "MIP パラメータ計測 ($W): $tag $*" && git push -q origin "HEAD:$BRANCH"
  echo "done $tag: $(grep SGM "$WORK/res_$tag.log")"
}
run base_a
run base_b
while read -r tag envs; do [ -n "$tag" ] && run "$tag" $envs; done < "$CFG"
echo ALL_DONE
