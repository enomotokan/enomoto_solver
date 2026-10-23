"""8 本に分けた並列の計測 (f970f87、newdef・mix・mixrole × HLa・HLb) をまとめる。
python -I agg_bench.py <json のディレクトリ> <分け方 (bench_subsets.txt)>"""
import json, math, sys, os, collections
d, subf = sys.argv[1], sys.argv[2]
sub = {}
for l in open(subf):
    t = l.split()
    if t and t[0].isdigit():
        for p in t[1:]:
            sub.setdefault(p, []).append(int(t[0]))
cfgs, sets = ["newdef", "mix", "mixrole"], ["HLa", "HLb"]
R = collections.defaultdict(dict)  # (cfg, set) -> {(suite, p): ours}
HI = {}
for f in os.listdir(d):
    if not f.endswith(".json"): continue
    suite = "40" if f.startswith("miplib_results") else "16"
    name = f.split("_f970f87_")[1][:-5]  # e.g. mix_s1_rHLa
    cfg, s, r = name.rsplit("_", 2)
    k = int(s[1:]); st = r[1:]
    for p, v in json.load(open(f"{d}/{f}")).items():
        if k in sub.get(p, []):
            R[(cfg, st)][(suite, p)] = v["ours"]
            HI[(suite, p)] = v["highs"]
keys = sorted(HI)
def pgap(o, ref):
    if o is None or ref is None: return 1.0
    if abs(o - ref) <= 1e-9 * max(1, abs(ref)): return 0.0
    if o * ref < 0: return 1.0
    return abs(o - ref) / max(abs(o), abs(ref))
# 参照値: HiGHS と全設定の目的値の最小 (どの問題も最小化として扱う。MIPLIB の元の向きが最大化の問題はこの 56 問にない前提)
ref = {}
for kk in keys:
    vals = [R[c][kk]["objective"] for c in R if kk in R[c] and R[c][kk]["objective"] is not None]
    if HI[kk]["objective"] is not None:
        vals.append(HI[kk]["objective"])
    ref[kk] = min(vals) if vals else None
def sgm(ts, shift):
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift
print("| 設定 | 組 | 40 問の解けた数 | 40 問の SGM 時間 (shift 10) | 56 問の主ギャップの和 | 最初の解の時刻の幾何平均 (shift 1、なしは 60) | 解なし | 不一致 |")
print("|---|---|---|---|---|---|---|---|")
for st in sets:
    for c in cfgs:
        rr = R[(c, st)]
        if len(rr) != len(keys):
            print(f"| {c} | {st} | 欠け {len(keys)-len(rr)} |"); continue
        k40 = [k for k in keys if k[0] == "40"]
        solved = sum(rr[k]["status"] == "optimal" for k in k40)
        t40 = sgm([min(rr[k]["time"], 60) if rr[k]["status"] == "optimal" else 60 for k in k40], 10)
        pg = sum(pgap(rr[k]["objective"], ref[k]) for k in keys)
        fs = sgm([rr[k]["first_solution_time"] if rr[k]["first_solution_time"] is not None else 60 for k in keys], 1)
        nosol = sum(rr[k]["objective"] is None for k in keys)
        wrong = [k[1] for k in keys if rr[k]["status"] == "optimal" and HI[k]["status"] == "Optimal" and abs(rr[k]["objective"] - HI[k]["objective"]) > 1e-6 * max(1, abs(HI[k]["objective"]))]
        print(f"| {c} | {st} | {solved} | {t40:.2f} | {pg:.3f} | {fs:.2f} | {nosol} | {len(wrong)} {wrong if wrong else ''} |")
# 問題ごと: 主ギャップが 2 組とも同じ向きに 0.01 以上変わった・解けた/解けないが変わった問題
print("\n問題ごと (newdef と比べて 2 組とも同じ向き。主ギャップ差 >= 0.01、解けた数の差、時間 1.5 倍)")
for c in ["mix", "mixrole"]:
    better, worse = [], []
    for k in keys:
        dg = [pgap(R[(c, s)][k]["objective"], ref[k]) - pgap(R[("newdef", s)][k]["objective"], ref[k]) for s in sets]
        so = [(R[(c, s)][k]["status"] == "optimal") - (R[("newdef", s)][k]["status"] == "optimal") for s in sets]
        tt = [(R[(c, s)][k]["time"] + 1) / (R[("newdef", s)][k]["time"] + 1) for s in sets]
        both_opt = all(R[(c, s)][k]["status"] == "optimal" and R[("newdef", s)][k]["status"] == "optimal" for s in sets)
        tag = f"{k[1]} (gap {['%.3f' % pgap(R[(c, s)][k]['objective'], ref[k]) for s in sets]} vs {['%.3f' % pgap(R[('newdef', s)][k]['objective'], ref[k]) for s in sets]}, 時間 {['%.1f' % R[(c, s)][k]['time'] for s in sets]} vs {['%.1f' % R[('newdef', s)][k]['time'] for s in sets]})"
        if all(x <= -0.01 for x in dg) or all(x > 0 for x in so) or (both_opt and all(x < 1 / 1.5 for x in tt)):
            better.append(tag)
        elif all(x >= 0.01 for x in dg) or all(x < 0 for x in so) or (both_opt and all(x > 1.5 for x in tt)):
            worse.append(tag)
    print(f"- {c} が良い ({len(better)}): " + "; ".join(better))
    print(f"- {c} が悪い ({len(worse)}): " + "; ".join(worse))
