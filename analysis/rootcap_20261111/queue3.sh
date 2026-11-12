#!/bin/sh
# ROOTCAP: queue2 の後に、w を抜いた hd を unitcal_7・net12・widden で 600 秒・種 1 (hdw で裏のカットの待ちを切ると終わりの下界が下がったため)
cd "$(dirname "$0")"
while pgrep -f queue2.sh >/dev/null; do sleep 20; done
for p in unitcal_7 net12 neos-4722843-widden; do
  grep -q "\"problem\": \"$p\", \"cfg\": \"hd\"" res.jsonl 2>/dev/null && continue
  python3 run1.py $p hd 1 600 >> q3.out 2>&1
done
