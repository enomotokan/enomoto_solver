#!/bin/bash
# ログの一覧: 名前 暫定解 下界 根のカット後 最初の解の時刻 ノード
cd "$(dirname "$0")/logs"
for f in ${1:-physiciansched6-2}_*.log; do
  j=$(grep '^{' $f | tail -1); [ -z "$j" ] && continue
  rc=$(grep -oE "Root cuts: objective [0-9.e+]+ -> [0-9.e+]+" $f | head -1 | awk '{print $NF}')
  echo "$f $(echo $j | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["status"], d["objective"], round(d["best_bound"],1) if d["best_bound"] is not None else None, d["first_solution_time"] and round(d["first_solution_time"],1), d["nodes"], round(d["time"],1))') rootcuts=$rc"
done
