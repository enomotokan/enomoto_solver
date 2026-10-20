#!/bin/bash
# summ.sh <log>...: 前処理後の大きさ・根・暫定解の推移・最後のノード数と下界
for f in "$@"; do echo "== $f"
grep -m1 "MIP: presolve:" $f | cut -c1-90
grep -m1 "^MIP: root LP" $f
grep "^Root cuts:" $f | head -2
grep -E "new incumbent" $f | awk '{printf "%s@%s%s ", $4, $8, $9}' ; echo
grep -E "^MIP: +[0-9.]+s nodes" $f | tail -1 | grep -oE "^MIP: +[0-9.]+s nodes +[0-9]+ open +[0-9]+ lb [^ ]+ ub [^ ]+|lp_rows [0-9]+|node_cut_secs [^ ]+|fc_sep calls.*extra [0-9]+" | tr '\n' ' '; echo
grep "tree search threads" $f | cut -c1-110
grep "^RESULT" $f | cut -c1-90
done
