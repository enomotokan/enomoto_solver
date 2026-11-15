"""CLIQFB の計測の集計 (analysis/cliqfb_20261111.md)。
python summ.py p58 [--cfg cp]   58 問 (p58_k*.jsonl): benchlib の report (SGM・95% 区間・解けた回・根の下界)、試しの判定、矛盾の検査
python summ.py cq600            8 問 × 600 秒 (cq600_k*.jsonl): 問題 x 種ごとの時間・暫定解・下界・根の下界・判定"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
args = sys.argv[1:]
B = "cp"
if "--cfg" in args:
    i = args.index("--cfg"); B = args[i + 1]; del args[i:i + 2]
kind = args[0] if args else "p58"
pre = {"p58": "p58_k", "p58r": "p58r_k", "p58r2": "p58r2_k", "cq600": "cq600_k", "cq600r": "cq600r_k", "cq600r2": "cq600r2_k", "p58f": "p58f_k", "cq600f": "cq600f_k", "p58f2": "p58f2_k"}[kind]
files = sorted(D.glob(pre + "*.jsonl"))
raw = [json.loads(l) for f in files for l in open(f) if l.strip()]


def decision(r):
    for l in r.get("clique", []):
        if "clique probe" in l or "clique fallback" in l:
            m = re.search(r"; (used \(fractional\)|used|not used)", l)
            if m:
                return m.group(1)
            if "no violated" in l:
                return "none"
    return "-"


if kind.startswith("p58"):
    R = bl.table(bl.load(files))
    keys = sorted(k for k in R if "def" in R[k] and B in R[k])
    try:
        noise = bl.load_noise()
    except Exception:  # noqa: BLE001
        noise = None
    bl.report(R, "def", B, keys, noise=noise, label=f"def → {B}")
else:
    TL = 600.0
    print(f"| 問題 | 種 | 設定 | 状態 | 時間 | 暫定解 | 下界 | 根の下界 | 判定 |")
    print("|---|---|---|---|---|---|---|---|---|")
    for r in sorted(raw, key=lambda r: (r["problem"], r["seed"], r["cfg"])):
        x = r["result"]
        g = lambda v: "-" if v is None else f"{v:.6g}"  # noqa: E731
        print(f"| {r['problem']} | {r['seed']} | {r['cfg']} | {x.get('status')} | {x.get('time', 0):.1f} | {g(x.get('objective'))} | {g(x.get('bound'))} | {g(x.get('root_bound'))} | {decision(r)} |")
# 矛盾の検査: 下界が (どの回の) 最良の暫定解を越える回、落ちた回
bad = 0
best = defaultdict(lambda: float("inf"))
for r in raw:
    z = r["result"].get("objective")
    if z is not None:
        best[r["problem"]] = min(best[r["problem"]], z)
for r in raw:
    res = r["result"]
    if res.get("status") == "crash":
        print("crash", r["problem"], r["cfg"], r["seed"]); bad += 1; continue
    b, bb = res.get("bound"), best[r["problem"]]
    if b is not None and b > bb + 1e-6 * max(1.0, abs(bb)):
        print("矛盾: 下界が最良の暫定解を越える", r["problem"], r["cfg"], r["seed"], b, bb); bad += 1
print(f"\n暫定解と下界の矛盾・落ちた回: {bad} (回 {len(raw)})")
# 試しの判定 (ログのある回だけ)
cnt = defaultdict(int)
for r in raw:
    if r["cfg"] == B:
        cnt[decision(r)] += 1
print("判定の数 (" + B + "):", dict(cnt))
