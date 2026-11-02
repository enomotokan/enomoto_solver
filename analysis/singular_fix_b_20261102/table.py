"""run.py のログから表を作る (Markdown を標準出力へ)。

使い方: python analysis/singular_fix_b_20261102/table.py [--set six|all58] [--configs base fix keep]
"""
import argparse
import json
import math
import re
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
import probs  # noqa: E402

SIX = ["neos5", "misc07", "noswot", "neos-911970", "swath1", "eil33-2", "roll3000", "timtab1"]
TREE = ("tree_sibling", "tree_queue", "tree_pool")


def parse(path):
    d = {"sites": {}, "fix": [0, 0, 0, 0, 0.0]}
    for l in path.read_text().splitlines():
        if l.startswith("RESULT "):
            d["res"] = json.loads(l[7:])
        elif l.startswith("NB_DIAG"):
            f = l.split()
            if f[1] != "main":
                continue
            kv = dict(x.split("=") for x in f[3:] if "=" in x)
            d["sites"][f[2]] = {k: float(v) for k, v in kv.items()}
        elif l.startswith("POOL_BASIS_FIX"):
            m = re.search(r"fixed (\d+) dropped (\d+) slacks (\d+) gave_up (\d+) secs ([\d.]+)", l)
            for i in range(4):
                d["fix"][i] += int(m.group(i + 1))
            d["fix"][4] += float(m.group(5))
    return d


def stats(d):
    r = d.get("res", {})
    s = d["sites"]
    g = lambda site, k: s.get(site, {}).get(k, 0.0)
    pool_n, pool_sing = g("tree_pool", "n"), g("tree_pool", "n_noeff")
    pool_it = g("tree_pool", "it_mis") + g("tree_pool", "it_ok")
    pool_t = g("tree_pool", "t_mis") + g("tree_pool", "t_ok")
    tree_it = sum(g(x, "it_mis") + g(x, "it_ok") for x in TREE)
    tree_sing = sum(g(x, "n_noeff") for x in TREE)
    all_it = sum(v.get("it_mis", 0) + v.get("it_ok", 0) for v in s.values())
    solved = r.get("status") == "optimal"
    t = r.get("time") if solved else 60.0
    return dict(status=r.get("status"), time=t, nodes=r.get("nodes") or 0, pool_n=pool_n, pool_sing=pool_sing, pool_it=pool_it, pool_t=pool_t,
                tree_it=tree_it, tree_sing=tree_sing, all_it=all_it, solved=solved, fix=d["fix"])


def sgm(xs, shift=10.0):
    return math.exp(sum(math.log(x + shift) for x in xs) / len(xs)) - shift


ap = argparse.ArgumentParser()
ap.add_argument("--set", choices=["six", "all58"], default="six")
ap.add_argument("--configs", nargs="*", default=["base", "fix", "keep"])
a = ap.parse_args()
names = SIX if a.set == "six" else probs.PROBS
runs = defaultdict(list)  # (config, prob) -> [stats]
for c in a.configs:
    for p in names:
        for f in sorted((HERE / "logs" / c).glob(f"{p}.r*.log")):
            runs[(c, p)].append(stats(parse(f)))

if a.set == "six":
    print("| 問題 | 設定 | 回 | 結果 (解けた回) | 時間 (各回) | ノード (各回) | pool の LP | うち特異 | pool の LP の反復 / LP | pool の LP 時間 | 木の set_basis の後の LP の反復 | main の全 LP の反復 | 直した回数・外した列 / 回・直す時間 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for p in names:
        for c in a.configs:
            rs = runs.get((c, p), [])
            if not rs:
                continue
            avg = lambda k: sum(x[k] for x in rs) / len(rs)
            fx = [sum(x["fix"][i] for x in rs) / len(rs) for i in range(5)]
            ptot = sum(x["pool_n"] for x in rs)
            fixs = f"{fx[0]:.0f}・{fx[1] / fx[0]:.1f}・{fx[4]:.3f} 秒" if fx[0] else "–"
            print(f"| {p} | {c} | {len(rs)} | {sum(x['solved'] for x in rs)}/{len(rs)} | {' / '.join(f'{x['time']:.1f}' for x in rs)} | "
                  f"{' / '.join(str(x['nodes']) for x in rs)} | {avg('pool_n'):.0f} | {avg('pool_sing'):.0f} | "
                  f"{(sum(x['pool_it'] for x in rs) / ptot if ptot else 0):.0f} | {avg('pool_t'):.2f} 秒 | {avg('tree_it'):.0f} | {avg('all_it'):.0f} | {fixs} |")
else:
    cs = a.configs
    common = [p for p in names if all(runs.get((c, p)) for c in cs)]
    print(f"問題 {len(common)} 問 (全設定にログのある問題)\n")
    print("| 設定 | 解けた | SGM 時間 (shift 10) | pool の LP の和 | うち特異 | 木の set_basis の後の LP の反復の和 | 木で特異 (全場所) |")
    print("|---|---|---|---|---|---|---|")
    for c in cs:
        rs = [runs[(c, p)][0] for p in common]
        print(f"| {c} | {sum(x['solved'] for x in rs)} | {sgm([x['time'] for x in rs]):.2f} | {sum(x['pool_n'] for x in rs):.0f} | "
              f"{sum(x['pool_sing'] for x in rs):.0f} | {sum(x['tree_it'] for x in rs):.0f} | {sum(x['tree_sing'] for x in rs):.0f} |")
    print("\n### 時間・結果が変わった問題 (時間の差 1 秒以上か 5% 以上、または解けた/解けないが違う)\n")
    print("| 問題 | " + " | ".join(f"{c} 時間 | {c} ノード | {c} pool 特異/LP" for c in cs) + " |")
    print("|---|" + "---|---|---|" * len(cs))
    for p in common:
        rs = [runs[(c, p)][0] for c in cs]
        t0, t1 = rs[0]["time"], rs[-1]["time"]
        if rs[0]["solved"] != rs[-1]["solved"] or abs(t1 - t0) >= max(1.0, 0.05 * t0) or rs[0]["pool_n"] > 0:
            print(f"| {p} | " + " | ".join(f"{x['time']:.1f}{'' if x['solved'] else ' (未)'} | {x['nodes']} | {x['pool_sing']:.0f}/{x['pool_n']:.0f}" for x in rs) + " |")
