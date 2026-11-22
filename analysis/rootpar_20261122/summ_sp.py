"""試作 ENOMOTO_MIP_SEP_PARALLEL (sp) と既定 (def) の 4 スレッドの比べ: python summ_sp.py > summ_sp.txt (res_sp.jsonl・res_sp_t1.jsonl)。
(問題, 種) の組ごとに、全体の秒・根の秒 (段 pre〜post の和、pre から起動〜求解の始めを引く)・カットのループの秒 (cut)・根の下界 (結果の root_bound)。
SGM は shift 1 秒 (根・カットは 0.1 秒)、時間切れは 120 秒で数え、CI は問題単位のブートストラップ 95%。"""
import json, math, random, collections
R = collections.defaultdict(dict)
for l in open("res_sp.jsonl"):
    r = json.loads(l); R[(r["problem"], r["seed"])][r["cfg"]] = r
PH = ["pre", "lp", "cut", "heur", "post"]
def load(r): return max(0.0, r["wall"] - (r["result"].get("time") or r["wall"]))
def sec(r, k):
    v = r["phases"].get(k, {}).get("secs", 0.0)
    return max(0.0, v - load(r)) if k == "pre" else v
def root(r): return sum(sec(r, k) for k in PH)
def solved(r): return str(r["result"].get("status")) in ("optimal", "infeasible")
def tot(r): return min(r["result"]["time"], 120.0) if solved(r) else 120.0
pairs = sorted(k for k, v in R.items() if "def" in v and "sp" in v)
def sgm(f, shift):
    byp = collections.defaultdict(list)
    for p, s in pairs:
        a, b = f(R[(p, s)]["sp"]), f(R[(p, s)]["def"])
        byp[p].append(math.log((a + shift) / (b + shift)))
    v = [sum(x) / len(x) for x in byp.values()]
    rnd = random.Random(1); bs = sorted(math.exp(sum(rnd.choice(v) for _ in v) / len(v)) for _ in range(2000))
    return math.exp(sum(v) / len(v)), bs[50], bs[1949], len(v)
print(f"組 {len(pairs)} (問題 {len({p for p, s in pairs})})")
st = collections.Counter((c, str(R[k][c]["result"].get("status"))) for k in pairs for c in ("def", "sp"))
print("状態:", dict(sorted(st.items())))
for name, f, sh in (("全体の秒", tot, 1.0), ("根の秒", root, 0.1), ("カットのループの秒", lambda r: sec(r, "cut"), 0.1)):
    e, lo, hi, n = sgm(f, sh)
    print(f"sp/def {name}: {e:.3f} [{lo:.3f}, {hi:.3f}] ({n} 問)")
s_def = sum(root(R[k]["def"]) for k in pairs); s_sp = sum(root(R[k]["sp"]) for k in pairs)
c_def = sum(sec(R[k]["def"], "cut") for k in pairs); c_sp = sum(sec(R[k]["sp"], "cut") for k in pairs)
print(f"合計: 根 {s_def:.0f} → {s_sp:.0f} 秒、カットのループ {c_def:.0f} → {c_sp:.0f} 秒")
# 根の下界・食い違い
same_rb = sum(1 for k in pairs if R[k]["def"]["result"].get("root_bound") == R[k]["sp"]["result"].get("root_bound"))
mis = [(k, R[k]["def"]["result"]["objective"], R[k]["sp"]["result"]["objective"]) for k in pairs
       if R[k]["def"]["result"].get("status") == "optimal" and R[k]["sp"]["result"].get("status") == "optimal"
       and abs(R[k]["def"]["result"]["objective"] - R[k]["sp"]["result"]["objective"]) > 1e-6 * max(1, abs(R[k]["def"]["result"]["objective"]))]
print(f"根の下界がちょうど同じ組: {same_rb}/{len(pairs)}、optimal 同士の目的値の食い違い: {mis or 'なし'}")
slow = [(k, round(tot(R[k]['def']), 1), round(tot(R[k]['sp']), 1)) for k in pairs if (tot(R[k]['sp']) + 1) / (tot(R[k]['def']) + 1) > 1.25]
fast = [(k, round(tot(R[k]['def']), 1), round(tot(R[k]['sp']), 1)) for k in pairs if (tot(R[k]['def']) + 1) / (tot(R[k]['sp']) + 1) > 1.25]
print("1.25 倍超の遅れ (ずらした比):", slow or "なし"); print("1.25 倍超の速さ:", fast or "なし")
print("\n| 問題 | 種 | 全体 def/sp | 根 def/sp | cut def/sp | cut の CPU/秒 def/sp | 根の下界 def / sp |")
print("|---|---|---|---|---|---|---|")
for p, s in pairs:
    a, b = R[(p, s)]["def"], R[(p, s)]["sp"]
    cpu = lambda r: (r["phases"].get("cut", {}).get("cpu", 0) / sec(r, "cut")) if sec(r, "cut") > 0.05 else 0
    print(f"| {p} | {s} | {tot(a):.1f}/{tot(b):.1f} | {root(a):.1f}/{root(b):.1f} | {sec(a, 'cut'):.1f}/{sec(b, 'cut'):.1f} | {cpu(a):.1f}/{cpu(b):.1f} | {a['result'].get('root_bound')} / {b['result'].get('root_bound')} |")
