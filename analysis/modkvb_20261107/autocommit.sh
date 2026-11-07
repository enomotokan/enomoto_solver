#!/bin/bash
# 計測の途中のデータを 15 分ごとにコミット・push する (コンテナの再起動に備える)
cd "$(dirname "$0")/../.."
while true; do
  sleep 900
  git add analysis/modkvb_20261107 >/dev/null 2>&1
  if ! git diff --cached --quiet; then
    git commit -qm "MODKVB: 計測の途中経過 ($(date +%H:%M))

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnGQXB3MfBehNtgGUt1"
    for i in 1 2 3 4 5; do git push -q -u origin ccr-1beed81b-ex0uhq 2>/dev/null && break; git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q origin/ccr-1beed81b-ex0uhq; sleep $((2**i)); done
  fi
done
