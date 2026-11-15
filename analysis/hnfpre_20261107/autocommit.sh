# 15 分ごとに計測の途中経過をコミット・push する (再起動で計測が止まっても結果が残るように)
cd /home/user/enomoto_solver
while true; do
  sleep 900
  git add analysis/hnfpre_20261107/*.jsonl 2>/dev/null
  git commit -qm "HNFPRE: 計測の途中経過 ($(date +%H:%M))" 2>/dev/null || continue
  for i in 1 2 3 4 5; do git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q origin/ccr-1beed81b-ex0uhq && git push -q origin ccr-1beed81b-ex0uhq && break; sleep 5; done
done
