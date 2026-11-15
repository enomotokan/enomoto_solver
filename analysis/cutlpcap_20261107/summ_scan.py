"""scan_*.jsonl から、根のカットのループでカットの LP が上限に当たった問題を表にする。使い方: python summ_scan.py scan_existing.jsonl"""
import json, sys
def gap_info(r):
    s = r.get("sense", 1); h = r.get("highs")
    e0, e1 = r.get("obj0"), r.get("obj1")
    if h is None or e0 is None or e1 is None:
        return None, None
    hi = s * h  # 内部 (最小化) の向き
    rel = (hi - e1) / max(abs(hi), 1.0)
    clo = (e1 - e0) / (hi - e0) if hi - e0 > 1e-9 * max(1, abs(hi)) else None
    return rel, clo
rows = []
for l in open(sys.argv[1]):
    r = json.loads(l)
    if not r["fails"]:
        continue
    rel, clo = gap_info(r)
    rr = r["rounds"]
    first_hit = r["fails"][0]
    d = rr[str(first_hit)]
    prev = [rr[k] for k in rr if int(k) < first_hit and "obj" in rr[k]]
    rows.append((r["problem"], r["tl"], r["rep"], r["end"], len([k for k in rr if "obj" in rr[k]]), first_hit, d.get("ncuts"), d.get("cap"), d.get("iters"), d.get("tot_gain"), d.get("action"),
                 r.get("root_lp_iters"), r["obj0"], r["obj1"], r.get("highs"), rel, clo))
print("problem tl rep end ok_rounds hit_round ncuts cap iters gain_from_start action rootLPit obj0 obj1 highs relgap closed_frac")
for x in sorted(rows):
    print(" ".join("-" if v is None else (f"{v:.4g}" if isinstance(v, float) else str(v)) for v in x))
