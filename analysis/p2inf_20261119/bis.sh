#!/bin/sh
# P2-INF: commit <sha> をビルドして neos859080 を 4 スレッド・種 <種,...> で回し、res_bis.jsonl に足す。
# sh bis.sh <sha> [秒=90] [種=1,2]。worktree・ビルド・展開した版は $BIS (既定は scratchpad の bis) に置く。 BISENV="名=値 ..." で環境変数を足す (結果の行に env として残す)。
set -e
D=$(cd "$(dirname "$0")" && pwd); R=$(cd "$D/../.." && pwd)
BIS=${BIS:-/tmp/claude-0/-home-user-enomoto-solver/73d0c415-6aab-5ce0-b6a6-d7bc7782e4ae/scratchpad/bis}; SHA=$(git -C "$R" rev-parse --short=9 "$1"); TL=${2:-90}; SEEDS=${3:-1,2}
mkdir -p "$BIS/pkg"
if [ ! -f "$BIS/pkg/$SHA/enomoto_solver/__init__.py" ]; then
  [ -d "$BIS/wt" ] || git -C "$R" worktree add --detach "$BIS/wt" "$SHA"
  git -C "$BIS/wt" checkout -q --detach "$SHA"
  rm -rf "$BIS/whl"; (cd "$BIS/wt" && CARGO_TARGET_DIR="$BIS/target" maturin build --release -q -o "$BIS/whl" >"$BIS/build_$SHA.log" 2>&1)
  mkdir -p "$BIS/pkg/$SHA"; (cd "$BIS/pkg/$SHA" && python3 -m zipfile -e "$BIS"/whl/*.whl .)
fi
for s in $(echo "$SEEDS" | tr , ' '); do
  t0=$(date +%s.%N)
  out=$(cd "$R" && env -u ENOMOTO_MIP_THREADS $BISENV PYTHONPATH="$BIS/pkg/$SHA" ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s \
        python3 scripts/run_miplib_benchmark.py --worker-ours "$R/.miplib_cache/neos859080.mps" --time-limit "$TL" --threads 4 2>/dev/null | grep '^{' | tail -1)
  w=$(python3 -c "import time;print(round(time.time()-$t0,1))")
  echo "{\"sha\":\"$SHA\",\"env\":\"$BISENV\",\"seed\":$s,\"tl\":$TL,\"wall\":$w,\"result\":${out:-null}}" | tee -a "$D/res_bis.jsonl"
done
