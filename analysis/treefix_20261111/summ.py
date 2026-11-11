"""TREEFIX の計測の集計 (analysis/treefix_20261111.md)。analysis/benchlib.py の report を使う。
python summ.py <jsonl ...> [--cfg B] [--base A]
- SGM・解けた回・95% 区間 (benchlib.report)
- 問題ごと: 時間 (解けなかった回は最終の相対ギャップ)、仕事の数・締めた整数列・新たな固定・死んだ未処理ノード・木の中の再スタート
- 暫定解と下界の矛盾 (どの回も: 下界 > 暫定解 + 許容、または設定の間で最適値と下界が食い違う) を数える"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl  # noqa: E402
args = sys.argv[1:]
A, B = "def", "tf"
if "--cfg" in args:
    i = args.index("--cfg"); B = args[i + 1]; del args[i:i + 2]
if "--base" in args:
    i = args.index("--base"); A = args[i + 1]; del args[i:i + 2]
raw = [json.loads(l) for f in args for l in open(f) if l.strip()]
R = bl.table(bl.load(args))
keys = sorted(k for k in R if A in R[k] and B in R[k])
try:
    noise = bl.load_noise()
except Exception:  # noqa: BLE001
    noise = None
bl.report(R, A, B, keys, noise=noise, label=f"{A} → {B}")
pat = re.compile(r"jobs (\d+) \((\S+)s.*integer columns tightened (\d+) \(newly fixed (\d+)\), dead open nodes (\d+)")
tfs = defaultdict(lambda: [0, 0.0, 0, 0, 0])
nres = defaultdict(int)
for r in raw:
    for l in r.get("treefix", []):
        m = pat.search(l)
        if m:
            e = tfs[(r["problem"], r["cfg"], r["seed"])]
            e[0] += int(m.group(1)); e[1] += float(m.group(2)); e[2] += int(m.group(3)); e[3] += int(m.group(4)); e[4] += int(m.group(5))
    nres[(r["problem"], r["cfg"], r["seed"])] = r.get("tree_restarts", 0)


def gap(res):
    z, b = res.get("objective"), res.get("bound")
    if z is None or b is None:
        return float("inf")
    return abs(z - b) / max(abs(z), abs(b), 1e-9)


def cell(res):
    return f"{res['time']:.1f}" if bl.ok(res) else f"({gap(res):.2%})"


print(f"\n| 問題 | 種 | {A} | {B} | 仕事 (秒) | 締めた列 | 新たな固定 | 死んだノード | 木の再スタート |")
print("|---|---|---|---|---|---|---|---|---|")
for (p, rep) in keys:
    s = [r["seed"] for r in raw if r["problem"] == p and r.get("rep", r["seed"] - 1) == rep][0]
    e = tfs.get((p, B, s), [0, 0.0, 0, 0, 0])
    if e[0] == 0 and bl.ok(R[(p, rep)][A]) == bl.ok(R[(p, rep)][B]) and abs(bl.t(R[(p, rep)][A]) - bl.t(R[(p, rep)][B])) < 0.1 * bl.t(R[(p, rep)][A]) + 1:
        continue
    print(f"| {p} | {s} | {cell(R[(p, rep)][A])} | {cell(R[(p, rep)][B])} | {e[0]} ({e[1]:.2f}) | {e[2]} | {e[3]} | {e[4]} | {nres.get((p, B, s), 0)} |")
# 矛盾の検査
bad = 0
best = defaultdict(lambda: float("inf"))
for r in raw:
    z = r["result"].get("objective")
    if z is not None:
        best[r["problem"]] = min(best[r["problem"]], z)
for r in raw:
    res = r["result"]
    z, b = res.get("objective"), res.get("bound")
    if res.get("status") == "crash":
        print("crash", r["problem"], r["cfg"], r["seed"]); bad += 1; continue
    bb = best[r["problem"]]
    if b is not None and b > bb + 1e-6 * max(1.0, abs(bb)):
        print("矛盾: 下界が最良の暫定解を越える", r["problem"], r["cfg"], r["seed"], b, bb); bad += 1
print(f"\n暫定解と下界の矛盾: {bad} (回 {len(raw)})")
