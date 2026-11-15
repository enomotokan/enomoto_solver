"""分担の結果を組ごとに ../res_t4_<組>[_<名前>].jsonl に分ける: python merge.py [名前] (名前なしは res_s*.jsonl。summ.py・quick.py で読める)"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
TAG = sys.argv[1] if len(sys.argv) > 1 else ""
rows = {}
for f in sorted(D.glob(f"res_{TAG}_s*.jsonl" if TAG else "res_s*.jsonl")):
    for l in f.read_text().splitlines():
        if l.strip():
            r = json.loads(l); rows.setdefault(r["cfg"], []).append(l)
for c, ls in rows.items():
    (D.parent / (f"res_t4_{c}_{TAG}.jsonl" if TAG else f"res_t4_{c}.jsonl")).write_text("".join(x + "\n" for x in ls)); print(c, len(ls))
