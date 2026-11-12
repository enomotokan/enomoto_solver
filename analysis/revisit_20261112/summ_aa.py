"""REVISIT の候補 6 件を、種 3 の基準を def と defz (既定をもう 1 回、res_Z_s0.jsonl) の幾何平均にして比べ直す。
種 3 の def が運悪く遅かった (A/A で defz が def より −8.6%) ので、その分を割り引くため。種 1・2 の基準は def のまま。
SGM は shift 1 秒、時間切れは制限時間で数える。95% 区間は問題単位のブートストラップ (4000 回)。
使い方: python summ_aa.py"""
import json, glob, math, random, collections
C6 = ["prepar", "ncm", "sbt", "ns250s", "lm2", "opf3"]
rows = [json.loads(l) for f in sorted(glob.glob("res_[PQZ]_s*.jsonl")) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def ok(r): return r["result"].get("status") in ("optimal", "infeasible")
def lt(r): return math.log((min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]) + 1)
probs = sorted({k[0] for k in R})
def base(p, s):
    a = R.get((p, "def", s))
    if s == 3 and (p, "defz", 3) in R: return (lt(a) + lt(R[(p, "defz", 3)])) / 2
    return lt(a)
def show(c, seeds, name):
    logr = collections.defaultdict(list)
    for p in probs:
        for s in seeds:
            if (p, "def", s) not in R or (p, c, s) not in R: continue
            # summ.py と同じく、どれも時間切れの対は数えない
            if not any(ok(R[k]) for k in [(p, c, s), (p, "def", s), (p, "defz", s)] if k in R): continue
            logr[p].append(lt(R[(p, c, s)]) - base(p, s))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v))
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    slow = [p for p in ps if len(seeds) >= 2 and len(logr[p]) == len(seeds) and all(x > math.log(1.25) for x in logr[p])]
    print(f"{name:6s} {c:7s} 対 {sum(len(v) for v in logr.values()):3d}  SGM {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]  揃って遅い {slow}")
for c in C6: show(c, [3], "種3")
for c in C6: show(c, [1, 2, 3], "種1〜3")
