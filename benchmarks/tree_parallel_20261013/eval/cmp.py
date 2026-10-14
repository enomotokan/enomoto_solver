"""JSONL を比べる: cmp.py A.jsonl[,A2...] B.jsonl[,B2...] ... (カンマ区切りは複数回の平均)。
- SGM: 共通の問題、未解決は 60 秒、shift 10 秒。複数回は問題ごとに時間の幾何平均を取ってから。
- 解けた: 回ごとの平均。
- ギャップ: 全ての設定で全ての回が時間切れの問題について、時間切れのときの相対ギャップ (%) の shift 1 の幾何平均
  (暫定解・下界がなければ 100%、上限 100%)。
- 除外: --skip name,name で問題を除く。"""
import json, math, sys
args = [a for a in sys.argv[1:] if not a.startswith("--")]
skip = set()
for a in sys.argv[1:]:
    if a.startswith("--skip="):
        skip = set(a[7:].split(","))
ok = lambda r: r.get("status") in ("optimal", "Optimal")
tm = lambda r: r["time"] if ok(r) and r.get("time") is not None else 60.0
def gap(r):
    o, b = r.get("objective"), r.get("bound")
    if o is None or b is None or not math.isfinite(b):
        return 100.0
    return min(100.0, 100 * abs(o - b) / max(abs(o), 1e-9))
def g(ts, s): return math.exp(sum(math.log(t + s) for t in ts) / len(ts)) - s
sets = [[{r["name"]: r for r in map(json.loads, open(f))} for f in a.split(",")] for a in args]
names = sorted(set.intersection(*[set(r) for runs in sets for r in runs]) - skip)
tout = [n for n in names if all(not ok(r[n]) for runs in sets for r in runs)]
base = None
for a, runs in zip(args, sets):
    per = [g([tm(r[n]) for r in runs], 10) for n in names]
    v = g(per, 10)
    gp = g([g([gap(r[n]) for r in runs], 1) for n in tout], 1) if tout else float("nan")
    solved = sum(ok(r[n]) for r in runs for n in names) / len(runs)
    base = base or (v, gp)
    lab = ",".join(x.split("/")[-1].replace(".jsonl", "") for x in a.split(","))
    print(f"{lab[:44]:44s} {len(names)}問 SGM {v:6.2f} ({100*(v/base[0]-1):+5.1f}%) 解けた {solved:4.1f} | 時間切れ{len(tout)}問のギャップ {gp:6.2f}% ({100*(gp/base[1]-1):+5.1f}%)")
