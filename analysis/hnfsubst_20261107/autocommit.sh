#!/bin/bash
# chain.sh が動いている間、15 分ごとに計測の途中経過をコミット・push する
cd /home/user/enomoto_solver
while pgrep -f chain.sh >/dev/null; do
  sleep 900
  n=$(cat analysis/hnfsubst_20261107/long600.jsonl analysis/hnfsubst_20261107/p58.jsonl 2>/dev/null | wc -l)
  git add analysis/hnfsubst_20261107 && git commit -q -m "HNFSUBST: 計測の途中経過 ($(date -u +%H:%M)、$n 行)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnErkhaisxYyDmeiLzV" && /tmp/claude-0/s/push.sh
done
