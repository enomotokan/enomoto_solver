"""run.py のログから表を作る (Markdown を標準出力へ)。

使い方: python analysis/pool_dse_20261102/table.py [--configs base fix dse both bothn]
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROBS = ["neos5", "misc07", "noswot", "neos-911970", "swath1", "eil33-2", "roll3000", "mc11"]
TREE = ("tree_sibling", "tree_queue", "tree_pool")


def parse(path):
    d = {"sites": {}, "dse": [0] * 6}
    for l in path.read_text().splitlines():
        if l.startswith("RESULT "):
            d["res"] = json.loads(l[7:])
        elif l.startswith("NB_DIAG"):
            f = l.split()
            if f[1] != "main":
                continue
            kv = dict(x.split("=") for x in f[3:] if "=" in x)
            d["sites"][f[2]] = {k: float(v) for k, v in kv.items()}
        elif l.startswith("POOL_DSE"):
            m = re.search(r"sent (\d+) nodes (\d+) weights .* received (\d+) applied (\d+) basic with weight (\d+) without (\d+)", l)
            for i in range(6):
                d["dse"][i] += int(m.group(i + 1))
    return d


def stats(d):
    r = d.get("res", {})
    s = d["sites"]
    g = lambda site, k: s.get(site, {}).get(k, 0.0)
    solved = r.get("status") == "optimal"
    return dict(
        time=r.get("time") if solved else 60.0, solved=solved, nodes=r.get("nodes") or 0,
        pool_n=g("tree_pool", "n"), pool_sing=g("tree_pool", "n_noeff"),
        pool_it=g("tree_pool", "it_mis") + g("tree_pool", "it_ok"),
        tree_it=sum(g(x, "it_mis") + g(x, "it_ok") for x in TREE), dse=d["dse"],
    )


ap = argparse.ArgumentParser()
ap.add_argument("--configs", nargs="*", default=["base", "fix", "dse", "both", "bothn"])
a = ap.parse_args()
runs = defaultdict(list)
for c in a.configs:
    for p in PROBS:
        for f in sorted((HERE / "logs" / c).glob(f"{p}.r*.log")):
            runs[(c, p)].append(stats(parse(f)))

print("### 各回\n")
print("| 問題 | 設定 | 解けた | 時間 (各回) | ノード (各回) | pool の LP (平均) | うち特異 | pool の反復 / LP | 木の反復 (平均) | 渡した重み / ノード | 重みあり / なしの基底変数 (受け取った 1 ノード平均) |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for p in PROBS:
    for c in a.configs:
        rs = runs.get((c, p), [])
        if not rs:
            continue
        avg = lambda k: sum(x[k] for x in rs) / len(rs)
        pn = sum(x["pool_n"] for x in rs)
        ds = [sum(x["dse"][i] for x in rs) for i in range(6)]
        wpn = f"{ds[1] / ds[0]:.0f}" if ds[0] else "–"
        wa = f"{ds[4] / ds[3]:.0f} / {ds[5] / ds[3]:.0f}" if ds[3] else "–"
        print(f"| {p} | {c} | {sum(x['solved'] for x in rs)}/{len(rs)} | {' / '.join(f'{x['time']:.1f}' for x in rs)} | {' / '.join(str(x['nodes']) for x in rs)} | "
              f"{avg('pool_n'):.0f} | {avg('pool_sing'):.0f} | {(sum(x['pool_it'] for x in rs) / pn if pn else 0):.0f} | {avg('tree_it') / 1000:.0f}k | {wpn} | {wa} |")

print("\n### 平均 (時間は解けなかった回を 60 秒として)\n")
print("| 問題 | " + " | ".join(f"{c} 時間 (解けた) | {c} pool 反復/LP | {c} 木の反復" for c in a.configs) + " |")
print("|---|" + "---|---|---|" * len(a.configs))
for p in PROBS:
    row = [p]
    for c in a.configs:
        rs = runs.get((c, p), [])
        if not rs:
            row += ["–"] * 3
            continue
        pn = sum(x["pool_n"] for x in rs)
        row.append(f"{sum(x['time'] for x in rs) / len(rs):.1f} ({sum(x['solved'] for x in rs)}/{len(rs)})")
        row.append(f"{(sum(x['pool_it'] for x in rs) / pn if pn else 0):.0f}")
        row.append(f"{sum(x['tree_it'] for x in rs) / len(rs) / 1000:.0f}k")
    print("| " + " | ".join(row) + " |")
