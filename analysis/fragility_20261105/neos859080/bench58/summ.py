"""bench58*.jsonl の集計 (python3 summ.py bench58b.jsonl logs): 解けた数・シフト 1 の幾何平均、差の大きい問題と試作が働いた問題。"""
import json, math, sys, gzip, os
f = sys.argv[1]; logd = sys.argv[2] if len(sys.argv) > 2 else None
R = {}
for l in open(f):
    d = json.loads(l); r = d["result"]
    ok = r.get("status") in ("optimal", "infeasible")
    R[(d["problem"], d["cfg"])] = (r.get("status"), r.get("time", 60) if ok else 60.0, ok)
probs = sorted({p for p, _ in R})
sg = lambda ts: math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1
for c in ["def", "rob"]:
    print(c, "解けた数", sum(R[(p, c)][2] for p in probs), "/", len(probs), "SGM(シフト 1) %.2f" % sg([R[(p, c)][1] for p in probs]))
def fired(p):
    if not logd: return ""
    fn = os.path.join(logd, f"{p}_rob.log.gz")
    if not os.path.exists(fn): return ""
    L = [l for l in gzip.open(fn, "rt") if "robust root IPM" in l]
    if not L: return ""
    return " / ".join(l.strip().replace("MIP: robust root IPM: ", "")[:110] for l in L)
print("| 問題 | 既定 (秒) | 試作 (秒) | 試作の働き |")
print("|---|---:|---:|---|")
for p in probs:
    a, b = R[(p, "def")], R[(p, "rob")]
    fs = fired(p)
    if fs or a[2] != b[2] or abs(a[1] - b[1]) > max(3, 0.25 * min(a[1], b[1])):
        g = lambda x: f"{x[1]:.1f}" if x[2] else "時間切れ"
        print(f"| {p} | {g(a)} | {g(b)} | {fs} |")
