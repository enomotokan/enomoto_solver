#!/bin/bash
# 計測の途中経過を 15 分ごとにコミット・push する (2026-11-07、HNFCUT)
cd "$(dirname "$0")/../.."
while true; do
  sleep 900
  git add analysis/hnfcut_20261107/*.jsonl analysis/hnfcut_20261107/*.log analysis/hnfcut_20261107/*.txt analysis/hnfcut_20261107/scan.jsonl 2>/dev/null
  if ! git diff --cached --quiet; then
    git commit -qm "HNFCUT: 計測の途中経過 ($(date +%H:%M))

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnHqAn5F8YJrUpVsvXP"
    for i in 1 2 3 4 5; do
      git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q --autostash origin/ccr-1beed81b-ex0uhq && git push -q origin HEAD:ccr-1beed81b-ex0uhq && break
      sleep $((i * 5))
    done
  fi
done
