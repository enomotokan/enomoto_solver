"""diag.jsonl (4 スレッドの def・mc3) と cands.json (構造) から、問題ごとの比べと条件の候補の当てはめを表にする。python summ.py > summ.txt"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C = {c["problem"]: c for c in json.loads((D / "cands.json").read_text())}
R = defaultdict(dict)
for l in (D / "diag.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["threads"] == 4: R[r["problem"]][(r["tag"], r["seed"])] = r


def score(r):
    """(解けたか, 秒, 相対ギャップ)。解けなければ秒は上限"""
    x = r["result"]; ok = x["status"] == "optimal"
    z, b = x.get("objective"), x.get("bound")
    gap = 0.0 if ok else (abs(z - b) / max(1.0, abs(z)) if z is not None and b is not None else math.inf)
    return ok, x["time"] if ok else r["tl"], gap


def feat(p):
    c = C[p]; s = [o for o in c["orbs"] if o["k"] in c["small"]]
    o0 = s[0]
    return dict(n_small=len(s), n_big=len(c["big"]), n_all=len(c["orbs"]), rc=round(sum(o["pk"] / o["cols"] for o in s) / len(s), 1),
                part=all(o["part"] == o["pk"] for o in s), packing_only=all(o["part"] == 0 for o in s),
                cover=round(sum(o["pk"] * o["cols"] for o in s) / o0["free"], 3),
                extra_per_row=round(sum(o["extra"] for o in s) / sum(o["pk"] for o in s), 1), cost0=all(o["cost"] == 0 for o in s))


rows = []
print("| 問題 | 種 | 既定 (状態・秒・ノード・ギャップ) | MC3 (同) | REPROBE の固定 (伝播+オービトープ+プロービング) | 静的にした数 / 固定列 |")
print("|---|---|---|---|---|---|")
for p in C:
    for s in (1, 2):
        a, b = R[p].get(("def", s)), R[p].get(("mc3", s))
        if not a or not b: continue
        fa, fb = score(a), score(b)
        rp = b["diag"].get("reprobe"); extra = sum(rp[:3]) if rp else 0
        def cell(r, f):
            x = r["result"]; return f'{x["status"][:4]} {x["time"]:.0f} {x["nodes"]} {f[2]:.2e}'
        print(f"| {p} | {s} | {cell(a, fa)} | {cell(b, fb)} | {extra} ({rp}) | {b['diag'].get('orbpp')} |")
        # 勝ち負け: 両方解けたら秒の比 (mc3/def)、片方だけ解けたらその側、両方解けなければギャップの比
        if fa[0] and fb[0]: v = fb[1] / fa[1]
        elif fa[0] != fb[0]: v = 0.5 if fb[0] else 2.0
        else: v = (fb[2] + 1e-9) / (fa[2] + 1e-9)
        rows.append(dict(problem=p, seed=s, ratio=v, reprobe=extra, **feat(p)))
print()
print("| 問題 | 小さいオービトープの数 | 大きい (10 列以上) | 行/列 | 分割 | パッキングだけ | 覆う割合 | 行あたりの外の列 | 費用 0 | REPROBE の固定 (種 1・2) | mc3/def (種 1・2) |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
by = defaultdict(list)
for r in rows: by[r["problem"]].append(r)
for p, rs in by.items():
    f = rs[0]
    print(f"| {p} | {f['n_small']} | {f['n_big']} | {f["rc"]} | {f["part"]} | {f['packing_only']} | {f['cover']} | {f['extra_per_row']} | {f['cost0']} | "
          f"{', '.join(str(r['reprobe']) for r in rs)} | {', '.join(f'{r['ratio']:.2f}' for r in rs)} |")
(D / "summ_rows.json").write_text(json.dumps(rows, indent=0))
