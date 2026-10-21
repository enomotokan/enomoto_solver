#!/bin/bash
# eno_dump.sh <prefix (dumps/..., 拡張子なし)> <ログ名> [環境変数=値 ...] -> logs/<ログ名>.log
# 書き出したカット LP を ENOMOTO の MIP 用 LP ラッパーで同じ出発の基底から解き直す (cut_lp_debug テスト)。
D="$(cd "$(dirname "$0")" && pwd)"
prefix=$1; name=$2; shift 2
cd "$D/../.." || exit 1
export RUSTFLAGS="-L /usr/lib/x86_64-linux-gnu -lpython3.13" LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
env CUTLP="$D/$prefix" ENOMOTO_DEBUG_EXT_ITERS=1 "$@" cargo test --release --lib cut_lp_debug -- --ignored --nocapture > "$D/logs/$name.log" 2>&1
echo "$name: $(grep 'CUTLP: rep' "$D/logs/$name.log" | tail -1 | sed 's/.*status=/status=/') $(grep 'degenerate_pivots=.*cost_shifts' "$D/logs/$name.log" | tail -1 | sed 's/DEBUG_EXT: //')"
