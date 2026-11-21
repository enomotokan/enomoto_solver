"""条件の候補ごとの得失の表。python cond.py > cond.txt
diag.jsonl の def と mc3 (4 スレッドの 13 問、1 スレッドの neos-911970・misc07) を種ごとに組にし、mc3 側の勝ち負けを決めて、
「その条件なら mc3 (静的) を選ぶ」問題の勝ち負けと、「選ばない」(既定のまま) 問題で避けた負け・逃した勝ちを数える。
勝ち負けの決め方 (mc3 から見て):
  両方解けた: 秒の比 mc3/def < 0.9 で勝ち、> 1.1 で負け、その間は分け (1.1 は揺れを分けにするための幅で、条件の閾値ではない)
  片方だけ解けた: 解けた側の勝ち (ab72 の def は根の LP の誤りで not_solved。別の不具合なので注を付けて勝ちに数える)
  両方解けない: 主 (目的値) と双対 (限界) を比べ、片方が良くもう片方が同じか良ければ勝ち、逆なら負け、食い違えば分け"""
import json
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C = {c["problem"]: c for c in json.loads((D / "cands.json").read_text())}
R = defaultdict(dict)
for l in (D / "diag.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["tag"] in ("def", "mc3"): R[(r["problem"], r["threads"])][(r["tag"], r["seed"])] = r
SENSE = {}  # 全て最小化
EPS = 1e-6


def verdict(a, b):
    x, y = a["result"], b["result"]
    oa, ob = x["status"] == "optimal", y["status"] == "optimal"
    if oa and ob:
        q = y["time"] / x["time"]
        return ("W" if q < 0.9 else "L" if q > 1.1 else "T"), f"秒 {x['time']:.0f}→{y['time']:.0f}"
    if oa != ob: return ("W" if ob else "L"), f"{x['status'][:4]}/{y['status'][:4]}"
    za, zb, ba, bb = x.get("objective"), y.get("objective"), x.get("bound"), y.get("bound")
    sc = max(1.0, abs(za or 1), abs(zb or 1))
    p = 0 if za is None or zb is None else (1 if zb < za - EPS * sc else -1 if zb > za + EPS * sc else 0)
    d = 0 if ba is None or bb is None else (1 if bb > ba + EPS * sc else -1 if bb < ba - EPS * sc else 0)
    s = f"主 {za:.6g}→{zb:.6g}, 双対 {ba:.6g}→{bb:.6g}"
    if p >= 0 and d >= 0 and (p or d): return "W", s
    if p <= 0 and d <= 0 and (p or d): return "L", s
    return "T", s


def feats(prob, th, pairs):
    a = pairs[("def", 1)]; b = pairs[("mc3", 1)]
    rpa = a["diag"].get("reprobe"); rpb = b["diag"].get("reprobe")
    new = (sum(rpb[:3]) if rpb else 0) - (sum(rpa[:3]) if rpa else 0)
    if prob in C:
        c = C[prob]; s = [o for o in c["orbs"] if o["k"] in c["small"]]
    else:  # 1 スレッドの 2 問は logs の ORBDIAG struct から
        import gzip, re
        t = gzip.open(D / "logs" / f"{prob}_s1_t1_def.log.gz", "rt").read()
        s = [dict(k=int(k), rows=int(r), cols=int(cc), pk=int(pk), part=int(pt), extra=int(ex), cost=float(co))
             for k, r, cc, pk, pt, ex, co in re.findall(r"ORBDIAG struct orbitope (\d+) (\d+)x(\d+) binary true local false packing_rows (\d+) partition_rows (\d+) extra_in_rows (\d+) .*abs_cost (\S+)", t)]
        s = [o for o in s if o["cols"] < 10]
    ks = {o["k"] for o in s}
    up = any(e[0] == "r0" and e[1] in ks and e[3] > 1e-6 for e in a["diag"].get("rootlp", []))
    return dict(new=new, n_small=len(s), part=all(o["part"] == o["pk"] for o in s), packing_only=all(o["part"] == 0 for o in s),
                cost0=all(o["cost"] == 0 for o in s), lp_up=up)


rows = []
for (prob, th), pairs in R.items():
    if ("def", 1) not in pairs or ("mc3", 1) not in pairs: continue
    f = feats(prob, th, pairs)
    vs = []
    for s in (1, 2):
        if ("def", s) in pairs and ("mc3", s) in pairs: vs.append(verdict(pairs[("def", s)], pairs[("mc3", s)]))
    rows.append(dict(problem=prob, threads=th, verdicts=vs, **f))

print("## 組ごとの勝ち負け (mc3 から見て)\n")
print("| 問題 | スレッド | 新しい固定 (REPROBE, mc3−def) | 小さいオービトープ | 分割行 | パッキングだけ | 費用 0 | 既定の根 LP が静的領域に重み | 種 1 | 種 2 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    print(f"| {r['problem']} | {r['threads']} | {r['new']} | {r['n_small']} | {r['part']} | {r['packing_only']} | {r['cost0']} | {r['lp_up']} | "
          + " | ".join(f"{v} ({s})" for v, s in r["verdicts"]) + " |")

CONDS = [
    ("全部 (MIN_COLS=3 そのまま)", lambda r: True),
    ("REPROBE で新しい固定が 1 つ以上", lambda r: r["new"] > 0),
    ("既定の根 LP が静的領域に重みを置く (静的固定が根の LP 解を切る)", lambda r: r["lp_up"]),
    ("小さいオービトープが 1 つだけ", lambda r: r["n_small"] == 1),
    ("行が全て分割行", lambda r: r["part"]),
    ("行が全てパッキング (分割行なし)", lambda r: r["packing_only"]),
    ("目的の係数が 0 でない", lambda r: not r["cost0"]),
]


def cnt(rs):
    c = {"W": 0, "L": 0, "T": 0}
    for r in rs:
        for v, _ in r["verdicts"]: c[v] += 1
    return c


print("\n## 条件ごとの得失 (組の数。選ぶ = 静的にする、選ばない = 既定の動的のまま)\n")
print("| 条件 | 選ぶ問題 | 選ぶ側 勝/負/分 | 選ばない問題 | 選ばない側で避けた負け | 逃した勝ち |")
print("|---|---|---|---|---|---|")
for name, fn in CONDS:
    yes = [r for r in rows if fn(r)]; no = [r for r in rows if not fn(r)]
    a, b = cnt(yes), cnt(no)
    print(f"| {name} | {len(yes)} | {a['W']}/{a['L']}/{a['T']} | {len(no)} ({', '.join(r['problem'] for r in no) or '-'}) | {b['L']} | {b['W']} |")
(D / "cond_rows.json").write_text(json.dumps(rows, indent=1, ensure_ascii=False))
