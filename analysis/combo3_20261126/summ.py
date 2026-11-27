"""combo3 (c2c v3) の第 1 段 (P) の集計 (protoscreen の summ.py の写し): python summ.py [res_s*.jsonl ...] → 標準出力。
試作ごとに、同じスレッドの数の既定 (def_t1 か def_t4) と、両方そろった (問題, 種) で比べる (analysis/slowm_20261119/ci.py と同じ計算):
- 比: ずらし幾何平均の比 (ずらし 10 秒、解けない回は上限 60 秒で数える)。問題ごとに種の平均の log をとり、問題の間の平均と 95% 区間 (1.96 × 標準誤差)。
- 解けた数 (既定 / 試作)、全種で 1.25 倍超の遅れ (素の時間の比か、ずらした比で、全種そろって 1.25 超) の問題。
- 食い違い: 試作と既定の最適値の不一致・最適と実行不能の衝突、MIPLIB の既知の値 (solu.json) との不一致 (試作の回だけ)。
- 採否の目安: 区間の上端 <= 1.05、解けた数が減らない、全種 1.25 倍超の遅れなし、食い違い 0 の全部を満たせば「候補」。
- 問題単位の比 (種の比の幾何平均、ずらし 10 秒) で、大きく効いた問題 (0.8 倍未満) と害の出た問題 (1.25 倍超) の数と名前。
  「条件付き候補」: 目安では外れだが、効いた問題と害の出た問題の両方があり (効き方が問題で分かれる)、食い違いが 0 のもの。
  利用者の方針 (2026-11-24): 問題で効き方が違う試作は一概に却下せず、問題の特徴の連続関数 (シグモイド) で効き方を変える候補にする。
- `--perprob` を付けると、試作ごと・スレッド数ごとの問題単位の比の一覧も出す。"""
import collections, glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS, P1  # noqa: E402
TL, SH = 60.0, 10.0
PERPROB = "--perprob" in sys.argv
files = [a for a in sys.argv[1:] if a != "--perprob"] or [str(D / "res_s*.jsonl")]
R = {}
for f in files:
    for g in glob.glob(f):
        for l in open(g, encoding="utf-8"):
            if l.strip():
                r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
SOLU = json.loads((D / "solu.json").read_text())
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"].get("time") or TL, TL) if solved(r) else TL
def obj(r):
    v = r["result"].get("objective")
    return v if isinstance(v, (int, float)) and math.isfinite(v) else None
def same(a, b): return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))
crash = [(p, c, s, st(r)) for (p, c, s), r in R.items() if st(r) in ("", "crash", "error", "fetch_error")]
print(f"行 {len(R)}、crash・取得失敗 {len(crash)} {crash[:10]}")
print(f"設定ごとの行の数: {dict(collections.Counter(c for _, c, _ in R))}\n")
# 害の分け方 (利用者の方針、README 7 節): kind.json の「道筋」の害は、採用の目安の「全種 1.25 倍超の遅れ」に数えず、別の欄に残す。載っていない問題は仕組みの害として数える
KIND = {k: v for k, v in json.loads((D / "kind.json").read_text()).items() if not k.startswith("_")}
def kind(p, th): return KIND.get(f"{p} {th}") or KIND.get(p) or ""
rows = []
for k, (name, env, t1, t4, rec) in P1.items():
    for th, on in (("t1", t1), ("t4", t4)):
        if not on: continue
        base, proto = f"def_{th}", f"{name}_{th}"
        by = collections.defaultdict(list)
        for (p, c, s) in R:
            if c == proto and (p, base, s) in R: by[p].append(s)
        if not by:
            rows.append((k, name, th, None)); continue
        logs, slow, slow_path, bad, nb, npr = [], [], [], [], 0, 0
        perprob = {}
        for p, ss in by.items():
            lr = [math.log((t(R[(p, proto, s)]) + SH) / (t(R[(p, base, s)]) + SH)) for s in ss]
            logs.append(sum(lr) / len(lr))
            perprob[p] = math.exp(sum(lr) / len(lr))
            nb += sum(solved(R[(p, base, s)]) for s in ss); npr += sum(solved(R[(p, proto, s)]) for s in ss)
            raw = [t(R[(p, proto, s)]) / max(t(R[(p, base, s)]), 1e-9) for s in ss]
            if all(x > 1.25 for x in raw) or all(math.exp(x) > 1.25 for x in lr):
                (slow_path if kind(p, th) in ("道筋", "揺れ") else slow).append(f"{p} {[round(x, 2) for x in raw]}")
            for s in ss:
                a, b = R[(p, proto, s)], R[(p, base, s)]
                if st(a) == "optimal" and st(b) == "optimal" and obj(a) is not None and obj(b) is not None and not same(obj(a), obj(b)):
                    bad.append(f"{p} 種 {s}: 最適値 {obj(a)} 対 既定 {obj(b)}")
                if {st(a), st(b)} == {"optimal", "infeasible"}:
                    bad.append(f"{p} 種 {s}: {st(a)} 対 既定 {st(b)}")
                sol = SOLU.get(p)
                if sol and sol["kind"] == "opt" and st(a) == "optimal" and obj(a) is not None and not same(obj(a), sol["value"]):
                    bad.append(f"{p} 種 {s}: 最適値 {obj(a)} 対 既知 {sol['value']}")
                if sol and sol["kind"] == "inf" and st(a) == "optimal":
                    bad.append(f"{p} 種 {s}: 最適 対 既知の実行不能")
        n = len(logs); m = sum(logs) / n
        sd = (sum((x - m) ** 2 for x in logs) / (n - 1)) ** 0.5 if n > 1 else 0.0
        h = 1.96 * sd / n ** 0.5
        good = sorted((p for p, x in perprob.items() if x < 0.8), key=lambda p: perprob[p])
        harm = sorted((p for p, x in perprob.items() if x > 1.25), key=lambda p: -perprob[p])
        rows.append((k, name, th, dict(n=n, ratio=math.exp(m), lo=math.exp(m - h), hi=math.exp(m + h), nb=nb, npr=npr, slow=slow, slow_path=slow_path, bad=bad,
                                       nrun=sum(len(v) for v in by.values()), perprob=perprob, good=good, harm=harm)))
# 前回の combo の害の切り分けで分かった原因 (analysis/combo_20261124/cause.json、キーは「問題」か「問題 t1」「問題 t4」)
CAUSE = json.loads((D.parent / "combo_20261124" / "cause.json").read_text())
def cause(p, th): return CAUSE.get(f"{p} {th}") or CAUSE.get(p) or ""
def plist(d, key, th=None):
    """効いた (good)・害の出た (harm) 問題の数と、名前と比の並び。害の問題には、前回の原因が分かっていれば短く付ける"""
    pp = d["perprob"]
    def one(p):
        c = cause(p, th) if key == "harm" and th else ""
        return p + " " + format(pp[p], ".2f") + (f" [前回: {c.split(' (')[0]}]" if c else "")
    return f"{len(d[key])}: " + (" ".join(one(p) for p in d[key]) or "—")


for th in ("t1", "t4"):
    print(f"== {'1' if th == 't1' else '4'} スレッド (既定 def_{th} との比、1 より小さければ試作が速い)\n")
    print("| 番号 | 試作 | 回 | 比 | 95% 区間 | 解けた数 既定 → 試作 | 全種 1.25 倍超の遅れ (仕組み・未分類) | 同 (道筋・揺れ、目安に数えない) | 食い違い | 目安 | 効いた (< 0.8) | 害 (> 1.25) | 条件付き候補 |")
    print("|---|---|---:|---:|---|---|---|---|---|---|---|---|---|")
    for k, name, t_, d in rows:
        if t_ != th: continue
        if d is None:
            print(f"| {k} | {name} | 0 | — | — | — | — | — | — | 結果なし | — | — | — |"); continue
        ok = d["hi"] <= 1.05 and d["npr"] >= d["nb"] and not d["slow"] and not d["bad"]
        why = [] if ok else [w for w, c in (("区間上端", d["hi"] > 1.05), ("解けた数", d["npr"] < d["nb"]), ("遅れ", bool(d["slow"])), ("食い違い", bool(d["bad"]))) if c]
        print(f"| {k} | {name} | {d['nrun']} | {d['ratio']:.3f} | [{d['lo']:.3f}, {d['hi']:.3f}] | {d['nb']} → {d['npr']} | {'; '.join(d['slow']) or 'なし'} | {'; '.join(d['slow_path']) or 'なし'} | "
              f"{'; '.join(d['bad']) or '0'} | {'候補' if ok else '外れ (' + '・'.join(why) + ')'} | "
              f"{plist(d, 'good')} | {plist(d, 'harm', th)} | "
              f"{'○' if (not ok and d['good'] and d['harm'] and not d['bad']) else ''} |")
    print()
if PERPROB:
    print("== 問題単位の比 (種の比の幾何平均、ずらし 10 秒。1 より小さければ試作が速い)\n")
    for k, name, th, d in rows:
        if d is None: continue
        pp = d["perprob"]
        print(f"{k} {name}_{th}: " + " ".join(f"{p} {pp[p]:.2f}" for p in sorted(pp, key=lambda q: pp[q])))

# c2c と c2a の問題ごとの比べ (親の依頼): 種ごとの比 (ずらし 10 秒) が、種をまたいで同じ向きか。
# 全種で c2c が遅い (比 > 1) か全種で速い (比 < 1) なら「同じ向き」(仕組みの見込み)、種で向きが変われば「変わる」(運の見込み)。
# 両方とも解けない (上限同士) の種は比が 1 なので向きに数えない。問題単位の比 (種の幾何平均) の大きい順に、1.25 倍超か 0.8 倍未満の問題を出す。
def pairdir(a_cfg, b_cfg, th):
    a, b = f"{a_cfg}_{th}", f"{b_cfg}_{th}"
    out = []
    for p in sorted({p for (p, c, s) in R if c == a}):
        ss = [s for s in (1, 2, 3) if (p, a, s) in R and (p, b, s) in R]
        if not ss: continue
        rs = [(t(R[(p, a, s)]) + SH) / (t(R[(p, b, s)]) + SH) for s in ss]
        g = math.exp(sum(math.log(x) for x in rs) / len(rs))
        sig = [x for x in rs if x != 1.0]
        d = "同じ向き" if sig and (all(x > 1 for x in sig) or all(x < 1 for x in sig)) else ("—" if not sig else "変わる")
        out.append((p, g, rs, d))
    return out
if any(c.startswith("c2c_") for (_, c, _) in R):
    for th in ("t1", "t4"):
        pd = pairdir("c2c", "c2a", th)
        if not pd: continue
        big = sorted([x for x in pd if x[1] > 1.25 or x[1] < 0.8], key=lambda x: -x[1])
        n_same = sum(1 for x in big if x[3] == "同じ向き")
        print(f"== c2c / c2a ({th}): 問題ごとの比が 1.25 倍超か 0.8 倍未満の {len(big)} 問のうち、種をまたいで同じ向き {n_same} 問\n")
        print("| 問題 | 比 (種の幾何) | 種ごとの比 | 向き | 前回の原因 |")
        print("|---|---:|---|---|---|")
        for p, g, rs, d in big:
            print(f"| {p} | {g:.2f} | {' / '.join(format(x, '.2f') for x in rs)} | {d} | {cause(p, th).split(' (')[0]} |")
        print()
