#!/bin/bash
# 途中のデータをコミットして push する。拒否されたら fetch・rebase (--autostash) して最大 5 回。衝突したら rebase をやめて知らせる
cd "$(dirname "$0")/../.."
git add -A analysis/modkvb_20261107 >/dev/null 2>&1
git diff --cached --quiet || git commit -qm "${1:-MODKVB: 計測の途中経過 ($(date +%H:%M))}

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnGQXB3MfBehNtgGUt1"
for i in 1 2 3 4 5; do
  git push -q -u origin ccr-1beed81b-ex0uhq 2>/dev/null && { echo "pushed $(git log --oneline -1)"; exit 0; }
  git fetch -q origin ccr-1beed81b-ex0uhq
  if ! git rebase -q --autostash origin/ccr-1beed81b-ex0uhq >/dev/null 2>&1; then git rebase --abort; echo "REBASE CONFLICT"; exit 1; fi
  sleep $((2**i))
done
echo "PUSH FAILED"; exit 1
