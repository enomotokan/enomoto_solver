"""cmp_*.jsonl の設定ごとの反復の合計・時間の合計・外れ値 (HiGHS の 3 倍超かつ 200 反復超) の数。使い方: python sum_cmp.py ファイル..."""
import json, sys
for f in sys.argv[1:]:
    rows = [json.loads(l) for l in open(f)]
    keys = [k for k in rows[0] if isinstance(rows[0][k], list) or rows[0][k] is None]
    keys = [k for k in rows[0] if k not in ("prefix", "kind", "depth", "rows", "tree_iters", "tree_status", "prim_inf")]
    out = [f"{f}: n {len(rows)} tree {sum(r['tree_iters'] for r in rows)}"]
    for k in keys:
        v = [r[k] for r in rows if r.get(k)]
        it = sum(x[0] for x in v); s = sum(x[1] for x in v)
        bad = sum(1 for r in rows if r.get(k) and r.get("highs") and r[k][0] > 3 * r["highs"][0] and r[k][0] > 200) if k != "highs" else 0
        out.append(f"{k} {it} ({s:.2f}s, >3xHiGHS {bad})")
    print(" | ".join(out))
