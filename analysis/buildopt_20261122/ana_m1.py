"""m1 (1 スレッド、15 問×2 回) の解析: 各版のノード数・目的値が base と同じかを確かめ、時間の比 (版/base) を問題ごとと幾何平均で出す。
使い方: python ana_m1.py [res_m1.jsonl]"""
import json, math, sys, collections
f = sys.argv[1] if len(sys.argv) > 1 else "res_m1.jsonl"
R = collections.defaultdict(dict)
for l in open(f):
    r = json.loads(l); R[(r["problem"], r["var"])][r["rep"]] = r["result"]
probs = sorted({p for p, _ in R}, key=lambda p: [r for r in open(f)].index(next(x for x in open(f) if json.loads(x)["problem"] == p)))
vars_ = []
for p, v in R:
    if v not in vars_: vars_.append(v)
def t(p, v):  # 2 回の最小
    return min(x["time"] for x in R[(p, v)].values())
print("== 同じ道筋か (nodes, objective, status が base の rep0 と同じ)")
bad = []
for p in probs:
    b = R[(p, "base")][0]
    for v in vars_:
        for rep, x in R[(p, v)].items():
            same = (x["nodes"], x["status"], x["objective"]) == (b["nodes"], b["status"], b["objective"])
            if not same: bad.append((p, v, rep, x["status"], x["nodes"], x["objective"], b["nodes"], b["objective"]))
for x in bad: print("  違う:", x)
print("  違い", len(bad), "件")
print("\n== 時間の比 (版/base、2 回の最小どうし)")
print("%-20s %7s " % ("problem", "base[s]") + " ".join("%7s" % v for v in vars_ if v != "base"))
lg = collections.defaultdict(list)
for p in probs:
    tb = t(p, "base"); row = []
    for v in vars_:
        if v == "base": continue
        q = t(p, v) / tb; lg[v].append(math.log(q)); row.append("%7.3f" % q)
    print("%-20s %7.1f " % (p, tb) + " ".join(row))
print("%-20s %7s " % ("幾何平均", "") + " ".join("%7.3f" % math.exp(sum(lg[v]) / len(lg[v])) for v in vars_ if v != "base"))
print("\n== 同じ版の 2 回の揺れ (|log(rep1/rep0)| の平均、%)")
for v in vars_:
    d = [abs(math.log(R[(p, v)][1]["time"] / R[(p, v)][0]["time"])) for p in probs if 1 in R[(p, v)]]
    print("  %-8s %.2f%%" % (v, 100 * sum(d) / len(d)))
