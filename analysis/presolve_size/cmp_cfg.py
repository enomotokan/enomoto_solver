"""既定・試作の設定・HiGHS の前処理後の形と LP 緩和の値を並べる (既定と違う問題だけ)。
使い方: python3 cmp_cfg.py sizes_final.jsonl e_<設定>.jsonl ..."""
import sys, json
base = {json.loads(l)["name"]: json.loads(l) for l in open(sys.argv[1])}
cfgs = [(f, {json.loads(l)["name"]: json.loads(l) for l in open(f)}) for f in sys.argv[2:]]
def s(e): return "—" if e is None else f"{e['rows']}×{e['cols']}, {e['nnz']}, LP {e['lp']:.6g}" if e['lp'] is not None else f"{e['rows']}×{e['cols']}, {e['nnz']}"
print("| 問題 | HiGHS | 既定 | " + " | ".join(f.replace('e_', '').replace('.jsonl', '') for f, _ in cfgs) + " |")
print("|---|---|---|" + "---|" * len(cfgs))
for nm in sorted(base):
    b = base[nm]["e2"]; row = [cf.get(nm, {}).get("e") for _, cf in cfgs]
    key = lambda e: None if e is None else (e["rows"], e["cols"], e["nnz"], None if e["lp"] is None else round(e["lp"], 6))
    if all(key(e) == key(b) for e in row):
        continue
    print(f"| {nm} | {s(base[nm]['h'])} | {s(b)} | " + " | ".join(s(e) + (f" ({cf[nm]['et']}s)" if e else "") for e, (_, cf) in zip(row, cfgs)) + " |")
