#!/bin/bash
# 途中の計測データをコミットして push する (拒否されたら fetch・rebase して最大 5 回)
cd "$(dirname "$0")/../.."
git add analysis/treesep_20261107 analysis/treesep_20261107.md src >/dev/null 2>&1
git commit -qm "木の中の分離の計測: 途中結果 (analysis/treesep_20261107)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnJgqzrJj7gHcceEXVw" >/dev/null 2>&1
for i in 1 2 3 4 5; do
  git push -q -u origin ccr-1beed81b-ex0uhq 2>/dev/null && exit 0
  git pull -q --rebase --autostash origin ccr-1beed81b-ex0uhq || git rebase --abort
  sleep $((2**i))
done
