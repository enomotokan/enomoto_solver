"""ふるい分けの集計: python summ.py [res_s*.jsonl ...] → 標準出力。
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
# res_wd_* を渡したら WARM=d の組 (s1wd・s2wd) で集計する (環境変数 POLISHP_SET=wd を付け忘れると「結果なし」になったため)
import os as _os  # noqa: E402
if any("res_wd_" in a for a in sys.argv[1:]):
    _os.environ.setdefault("POLISHP_SET", "wd")
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
        logs, slow, bad, nb, npr = [], [], [], 0, 0
        perprob = {}
        for p, ss in by.items():
            lr = [math.log((t(R[(p, proto, s)]) + SH) / (t(R[(p, base, s)]) + SH)) for s in ss]
            logs.append(sum(lr) / len(lr))
            perprob[p] = math.exp(sum(lr) / len(lr))
            nb += sum(solved(R[(p, base, s)]) for s in ss); npr += sum(solved(R[(p, proto, s)]) for s in ss)
            raw = [t(R[(p, proto, s)]) / max(t(R[(p, base, s)]), 1e-9) for s in ss]
            if all(x > 1.25 for x in raw) or all(math.exp(x) > 1.25 for x in lr):
                slow.append(f"{p} {[round(x, 2) for x in raw]}")
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
        rows.append((k, name, th, dict(n=n, ratio=math.exp(m), lo=math.exp(m - h), hi=math.exp(m + h), nb=nb, npr=npr, slow=slow, bad=bad,
                                       nrun=sum(len(v) for v in by.values()), perprob=perprob, good=good, harm=harm)))
def plist(d, key):
    """効いた (good)・害の出た (harm) 問題の数と、名前と比の並び"""
    pp = d["perprob"]
    return f"{len(d[key])}: " + (" ".join(p + " " + format(pp[p], ".2f") for p in d[key]) or "—")


for th in ("t1", "t4"):
    print(f"== {'1' if th == 't1' else '4'} スレッド (既定 def_{th} との比、1 より小さければ試作が速い)\n")
    print("| 番号 | 試作 | 回 | 比 | 95% 区間 | 解けた数 既定 → 試作 | 全種 1.25 倍超の遅れ | 食い違い | 目安 | 効いた (< 0.8) | 害 (> 1.25) | 条件付き候補 |")
    print("|---|---|---:|---:|---|---|---|---|---|---|---|---|")
    for k, name, t_, d in rows:
        if t_ != th: continue
        if d is None:
            print(f"| {k} | {name} | 0 | — | — | — | — | — | 結果なし | — | — | — |"); continue
        ok = d["hi"] <= 1.05 and d["npr"] >= d["nb"] and not d["slow"] and not d["bad"]
        why = [] if ok else [w for w, c in (("区間上端", d["hi"] > 1.05), ("解けた数", d["npr"] < d["nb"]), ("遅れ", bool(d["slow"])), ("食い違い", bool(d["bad"]))) if c]
        print(f"| {k} | {name} | {d['nrun']} | {d['ratio']:.3f} | [{d['lo']:.3f}, {d['hi']:.3f}] | {d['nb']} → {d['npr']} | {'; '.join(d['slow']) or 'なし'} | "
              f"{'; '.join(d['bad']) or '0'} | {'候補' if ok else '外れ (' + '・'.join(why) + ')'} | "
              f"{plist(d, 'good')} | {plist(d, 'harm')} | "
              f"{'○' if (not ok and d['good'] and d['harm'] and not d['bad']) else ''} |")
    print()
if PERPROB:
    print("== 問題単位の比 (種の比の幾何平均、ずらし 10 秒。1 より小さければ試作が速い)\n")
    for k, name, th, d in rows:
        if d is None: continue
        pp = d["perprob"]
        print(f"{k} {name}_{th}: " + " ".join(f"{p} {pp[p]:.2f}" for p in sorted(pp, key=lambda q: pp[q])))
