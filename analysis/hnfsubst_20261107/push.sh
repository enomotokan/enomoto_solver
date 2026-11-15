#!/bin/bash
cd /home/user/enomoto_solver
for i in 1 2 3 4 5; do
  git push -q -u origin ccr-1beed81b-ex0uhq 2>/dev/null && { echo pushed; exit 0; }
  git fetch -q origin ccr-1beed81b-ex0uhq && git rebase -q --autostash origin/ccr-1beed81b-ex0uhq || { echo REBASE_FAIL; exit 1; }
  sleep $((2**i))
done
echo PUSH_FAIL; exit 1
