#!/bin/bash
# 計測の間、15 分ごとに analysis/tree_restart_guard_20261106 をコミット・push する (コンテナの入れ替わりに備える)
cd "$(dirname "$0")/../.."
while true; do
  sleep 900
  git add -A analysis/tree_restart_guard_20261106 analysis/tree_restart_guard_20261106.md 2>/dev/null
  git diff --cached --quiet && continue
  git commit -qm "tree restart ガード: 計測の途中データ (自動)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnFQi4DKbTKEQoJfX8Y"
  for i in 1 2 3 4 5; do git push -q -u origin ccr-1beed81b-ex0uhq 2>/dev/null && break; git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q --autostash origin/ccr-1beed81b-ex0uhq; sleep $((2**i)); done
done
