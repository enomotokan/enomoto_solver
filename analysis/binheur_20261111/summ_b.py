"""BINHEUR の計測の集計 (analysis/binheur_20261111.md)。analysis/benchlib.py の report を使う。
python summ_b.py <jsonl ...> [--cfg swap] [--base def]
- SGM・解けた回・95% 区間 (benchlib.report)
- 問題ごと: 時間 (解けなかった回は最終の相対ギャップ)、最初の解・最良既知に届いた時刻・最終の解の値
- 暫定解と下界の矛盾 (下界が全設定の最良の暫定解を越える) と crash を数える"""
import json, sys
from collections import defaultdict
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl  # noqa: E402
args = sys.argv[1:]
A, B = "def", "swap"
if "--cfg" in args:
    i = args.index("--cfg"); B = args[i + 1]; del args[i:i + 2]
if "--base" in args:
    i = args.index("--base"); A = args[i + 1]; del args[i:i + 2]
WT = Path(__file__).resolve().parents[2]
SOLU = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        SOLU[s[1]] = float(s[2])
raw = [json.loads(l) for f in args for l in open(f) if l.strip()]
R = bl.table(bl.load(args))
keys = sorted(k for k in R if A in R[k] and B in R[k])
try:
    noise = bl.load_noise()
except Exception:  # noqa: BLE001
    noise = None
bl.report(R, A, B, keys, noise=noise, label=f"{A} → {B}")
I = {(r["problem"], r["cfg"], r["seed"]): r for r in raw}


def close(a, b): return a is not None and b is not None and abs(a - b) <= 1e-6 * max(1.0, abs(b))


def gap(res):
    z, b = res.get("objective"), res.get("bound")
    if z is None or b is None:
        return float("inf")
    return abs(z - b) / max(abs(z), abs(b), 1e-9)


def cell(res):
    return f"{res['time']:.1f}" if bl.ok(res) else f"({gap(res):.2%})"


def topt(r):
    ref = SOLU.get(r["problem"])
    return next((t for z, t, _ in r["incs"] if close(z, ref)), None)


f = lambda v: "-" if v is None else f"{v:.1f}"
print(f"\n| 問題 | 種 | {A} | {B} | 最初の解 {A} / {B} | 最良既知に届く {A} / {B} | 最終の解 {A} / {B} |")
print("|---|---|---|---|---|---|---|")
for (p, s) in keys:
    ra, rb_ = I[(p, A, s)], I[(p, B, s)]
    a, b = R[(p, s)][A], R[(p, s)][B]
    if bl.ok(a) == bl.ok(b) and abs(bl.t(a) - bl.t(b)) < 0.1 * bl.t(a) + 1 and close(a.get("objective"), b.get("objective")):
        continue
    print(f"| {p} | {s} | {cell(a)} | {cell(b)} | {f(a.get('first_solution_time'))} / {f(b.get('first_solution_time'))} | "
          f"{f(topt(ra))} / {f(topt(rb_))} | {a.get('objective')} / {b.get('objective')} |")
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
    if res.get("status") == "optimal" and r["problem"] in SOLU and not close(z, SOLU[r["problem"]]):
        print("誤答?: 最適値が最良既知と違う", r["problem"], r["cfg"], r["seed"], z, SOLU[r["problem"]]); bad += 1
print(f"\n暫定解と下界の矛盾・誤答・crash: {bad} (回 {len(raw)})")
