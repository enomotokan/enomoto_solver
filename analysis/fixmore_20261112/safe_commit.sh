#!/bin/sh
# 計測中の live ファイル (追跡しない) の行を res_sK.jsonl に写してコミットし push する
# (pull --rebase の間に追記されて行を失う事故を避ける)。使い方: sh safe_commit.sh "メッセージ"
cd "$(dirname "$0")/../.."

# live ファイルの行を res_sK.jsonl に写す (まだないものだけ)
python3 - <<'P'
import glob
for lv in glob.glob("analysis/fixmore_20261112/res*.live.jsonl"):
    f = lv.replace(".live", "")
    have = set(open(f).read().splitlines()) if __import__("os").path.exists(f) else set()
    new = [l for l in open(lv).read().splitlines() if l.strip() and l not in have]
    if new:
        open(f, "a").write("\n".join(new) + "\n")
P
mkdir -p analysis/fixmore_20261112/logs && mv analysis/fixmore_20261112/logs_live/* analysis/fixmore_20261112/logs/ 2>/dev/null
git add analysis/fixmore_20261112 analysis/fixmore_20261112.md docs/backlog.md 2>/dev/null
git commit -qm "$1

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnJbzZ4bTGrkAZ1tXBa"
for i in 1 2 3 4; do git pull -q --rebase origin ccr-1beed81b-ex0uhq && git push -q -u origin ccr-1beed81b-ex0uhq && break; sleep $((2 * i)); done
git status | sed -n 2p
