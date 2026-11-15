"""分担の結果を ../res_t4_base2.jsonl・../res_t4_spgq.jsonl の形に分ける: python merge.py (summ.py・quick.py で読める)"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {"base2": [], "spgq": []}
for f in sorted(D.glob("res_s*.jsonl")):
    for l in f.read_text().splitlines():
        if l.strip():
            r = json.loads(l); rows[r["cfg"]].append(l)
for c, ls in rows.items():
    (D.parent / f"res_t4_{c}.jsonl").write_text("".join(x + "\n" for x in ls)); print(c, len(ls))
