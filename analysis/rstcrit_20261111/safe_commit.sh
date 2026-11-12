#!/bin/sh
# 計測中の結果ファイルに 1 行足された直後 (次の書き込みまで約 60 秒ある) にコミットして push する
# (pull --rebase の間に追記されて行を失う事故を避ける)。使い方: sh safe_commit.sh "メッセージ"
cd "$(dirname "$0")/../.."
cnt() { cat analysis/rstcrit_20261111/res_s*.jsonl 2>/dev/null | wc -l; }
n=$(cnt)
if pgrep -f run_share.py >/dev/null; then
  until [ "$(cnt)" -gt "$n" ]; do sleep 2; done
fi
git add analysis/rstcrit_20261111 analysis/rstcrit_20261111.md docs/backlog.md 2>/dev/null
git commit -qm "$1

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnEqoPVMcdigNnHGwqu"
for i in 1 2 3 4; do git pull -q --rebase origin ccr-1beed81b-ex0uhq && git push -q -u origin ccr-1beed81b-ex0uhq && break; sleep $((2 * i)); done
git status | sed -n 2p
