"""過去の計測 (halve_submip_20261030 の bench*.jsonl) で、「最適」と返した目的関数値が問題ごとに食い違っていないかを調べる。
使い方: python optcheck.py [パターン ...] (既定は bench7・bench8)。相対 1e-6 を越える違いを食い違いとする。
参照値は benchmarks/ と analysis/ のすべての jsonl・json で最も多く「最適」と返された値。"""
import glob, json, sys
from collections import Counter, defaultdict
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
pats = sys.argv[1:] or ["analysis/halve_submip_20261030/bench7_s*.jsonl", "analysis/halve_submip_20261030/bench8_s*.jsonl"]

def rows(path):
    try:
        if path.endswith(".jsonl"):
            for l in open(path):
                l = l.strip()
                if l.startswith("{"):
                    try:
                        yield json.loads(l)
                    except Exception:
                        pass
        else:
            d = json.load(open(path))
            if isinstance(d, dict) and all(isinstance(v, dict) and ("ours" in v or "highs" in v) for v in d.values()):
                # benchmarks/miplib_results_*.json: {問題: {"highs": 結果, "ours": 結果}}
                for p, v in d.items():
                    for who in ("highs", "ours"):
                        if isinstance(v.get(who), dict):
                            yield {"problem": p, "cfg": who, "result": v[who]}
                return
            if isinstance(d, dict):
                d = d.get("results", d.get("rows", []))
            if isinstance(d, list):
                yield from (x for x in d if isinstance(x, dict))
    except Exception:
        return

def opt_of(r):
    res = r.get("result") or r.get("res") or r.get("ours") or r
    if not isinstance(res, dict):
        return None, None
    p = r.get("problem") or r.get("prob") or r.get("name")
    if res.get("status") in ("optimal", "Optimal", "Optimal solution found") and isinstance(res.get("objective"), (int, float)):
        return p, float(res["objective"])
    return p, None

def close(a, b):
    return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))

ref = defaultdict(Counter)
for f in glob.glob(str(WT / "benchmarks" / "*.json*")) + glob.glob(str(WT / "analysis" / "**" / "*.jsonl"), recursive=True):
    for r in rows(f):
        p, v = opt_of(r)
        if p and v is not None:
            ref[p][float(f"{v:.9g}")] += 1
REF = {p: c.most_common(1)[0][0] for p, c in ref.items()}

if __name__ == "__main__":
    n = 0
    bad = []
    for pat in pats:
        for f in sorted(glob.glob(str(WT / pat))):
            for r in rows(f):
                p, v = opt_of(r)
                if p is None or v is None:
                    continue
                n += 1
                if p in REF and not close(v, REF[p]):
                    bad.append((Path(f).name, p, r.get("cfg"), r.get("rep"), v, REF[p]))
    print(f"optimal rows: {n}, mismatches: {len(bad)}")
    for b in bad:
        print(*b)
    # 参照値自体に複数の値がある問題
    for p, c in sorted(ref.items()):
        vals = sorted(c)
        if len(vals) > 1 and not all(close(vals[0], v) for v in vals):
            print("ref-multi", p, dict(c))
