#!/bin/bash
# 計測の途中データ (このディレクトリ) をコミットして push する (拒否されたら fetch・rebase して最大 5 回)
cd "$(dirname "$0")"
git add -A . >/dev/null 2>&1
git diff --cached --quiet && exit 0
git commit -qm "CUTLPCAP: 計測の途中経過 ($(date +%H:%M))

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnG8MatpKRTr9P1JNJQ" -- . 
for i in 1 2 3 4 5; do
  git push -q origin ccr-1beed81b-ex0uhq 2>/dev/null && exit 0
  git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q --autostash origin/ccr-1beed81b-ex0uhq || git rebase --abort
  sleep 3
done
