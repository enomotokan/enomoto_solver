#!/bin/bash
# 計測中の jsonl・済んだログの写しをコミットして push する (書いている途中のファイルは写しだけ。git rebase で書き直されないように)
cd "$(dirname "$0")/../.."
D=analysis/mip_lp_scale_20261108
mkdir -p $D/snap
for f in $D/*.jsonl; do [ -f "$f" ] && cp "$f" $D/snap/; done
git add -f $D/snap $D/snap.sh; git diff --cached --quiet || git commit -qm "計測の途中 (改良 11-08 の写し)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01HrNYnFuWAnewBxp76Vi6t8"
for i in 1 2 3 4 5; do git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q origin/ccr-1beed81b-ex0uhq && git push -q -u origin ccr-1beed81b-ex0uhq 2>&1 && { echo pushed; break; }; sleep 3; done
