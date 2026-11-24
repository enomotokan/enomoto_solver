"""combo の計測の集計: python analysis/combo_20261124/summ.py [res_s*.jsonl ...] → 標準出力 (README の表の元)。
combo_tN を既定 def_tN と、両方そろった (問題, 種) で比べる (analysis/slowm_20261119/ci.py・protoscreen の summ.py と同じ計算)。スレッド別 (1・4) と
集合別 (M 186 問・240 問全部・残りの 54 問、problems.json) に出す:
- 比: ずらし幾何平均の比 (ずらし 10 秒、解けない回 (時間切れ・killed・crash) は上限 600 秒で数える)。問題ごとの log の比の平均と 95% 区間 (1.96 × 標準誤差)。
  種は 1 つなので、区間は問題の間のばらつきだけ (同じ問題の回の揺れは入らない)。
- 解けた数 (既定 → combo) と、片方だけ解けた問題。
- 食い違い: combo と既定の最適値の不一致・最適と実行不能の衝突、MIPLIB の既知の値 (solu.json) との不一致 (既定・combo の両方の回)。
- 1.25 倍超の遅れの問題 (ずらした比 > 1.25、種 1 つ) と、効いた問題 (ずらした比 < 0.8) の一覧 (時間と状態つき)。
- 目安 (protoscreen と同じ): 区間の上端 <= 1.05、解けた数が減らない、1.25 倍超の遅れなし、食い違い 0。新しい閾値は作らない。"""
import collections, glob, json, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
TL, SH = 600.0, 10.0
files = sys.argv[1:] or [str(D / "res_s*.jsonl")]
R = {}
for f in files:
    for g in glob.glob(f):
        for l in open(g, encoding="utf-8"):
            if l.strip():
                r = json.loads(l); key = (r["problem"], r["cfg"], r["seed"])
                # 同じ回の行が複数あれば (やり直し・組み替えの重なり)、crash・取得失敗でない行を優先し、その中では後の行
                if key not in R or str(r["result"].get("status") or "").lower() not in ("crash", "fetch_error"):
                    R[key] = r
PJ = json.loads((D / "problems.json").read_text())
SETS = [("M 186 問", set(PJ["M"])), ("240 問全部", set(PJ["all"])), ("残りの 54 問", set(PJ["rest"]))]
SOLU = json.loads((D / "solu.json").read_text())
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"].get("time") or TL, TL) if solved(r) else TL
def obj(r):
    v = r["result"].get("objective")
    return v if isinstance(v, (int, float)) and math.isfinite(v) else None
def same(a, b): return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))
def desc(r): return f"{st(r)} {t(r):.1f}s"
print(f"行 {len(R)}、設定ごと {dict(collections.Counter(c for _, c, _ in R))}、コミット {dict(collections.Counter(r.get('commit') for r in R.values()))}")
odd = [(p, c, s, st(r)) for (p, c, s), r in R.items() if st(r) in ("", "crash", "killed", "error", "fetch_error")]
print(f"crash・killed・取得失敗 {len(odd)}: {odd}")

# 食い違い (全部の行で)
bad = []
for (p, c, s), r in R.items():
    sol = SOLU.get(p); o = obj(r)
    if sol:
        if sol["kind"] == "inf" and (st(r) == "optimal" or o is not None):
            bad.append(f"{p} {c}: 既知は実行不能なのに {st(r)} {o}")
        if sol["kind"] == "opt" and st(r) == "infeasible":
            bad.append(f"{p} {c}: 既知の最適値 {sol['value']} があるのに実行不能")
        if sol["kind"] == "opt" and st(r) == "optimal" and o is not None and not same(o, sol["value"]):
            bad.append(f"{p} {c}: 最適値 {o} が既知の {sol['value']} と違う")
        if sol["kind"] in ("opt", "best") and o is not None and sol["value"] is not None and o < sol["value"] - 1e-6 * max(1.0, abs(sol["value"])):
            bad.append(f"{p} {c}: 解 {o} が既知の値 {sol['value']} より良い (MIPLIB は最小化)")
for th in ("t1", "t4"):
    for (p, c, s), a in R.items():
        if c != f"combo_{th}" or (p, f"def_{th}", s) not in R: continue
        b = R[(p, f"def_{th}", s)]
        if st(a) == "optimal" and st(b) == "optimal" and obj(a) is not None and obj(b) is not None and not same(obj(a), obj(b)):
            bad.append(f"{p} {th}: combo の最適値 {obj(a)} 対 既定 {obj(b)}")
        if {st(a), st(b)} == {"optimal", "infeasible"}:
            bad.append(f"{p} {th}: 最適と実行不能 (combo {st(a)}、既定 {st(b)})")
print(f"\n== 食い違い {len(bad)} 件")
for x in bad: print("  ", x)

for th, name in (("t1", "1 スレッド"), ("t4", "4 スレッド")):
    base, cmb = f"def_{th}", f"combo_{th}"
    print(f"\n== {name} (combo_{th} / def_{th}、1 より小さければ combo が速い)\n")
    print("| 集合 | 組 | 比 | 95% 区間 | 解けた数 既定 → combo | 既定だけ | combo だけ | 1.25 倍超の遅れ | 0.8 倍未満 | 目安 |")
    print("|---|---:|---:|---|---|---:|---:|---:|---:|---|")
    detail = {}
    # 記録が無効な回 (crash・killed・取得失敗。台のメモリ不足の OOM もここに入る) が def と combo の両方である組は、
    # ふだんは解けなかった回 (上限の秒) として数える。比の計算から外した形も、240 問全部と M に併記する。
    inv = lambda r: st(r) in ("", "crash", "killed", "error", "fetch_error")
    both_inv = {p for (p, c, s) in R if c == cmb and (p, base, s) in R and inv(R[(p, cmb, s)]) and inv(R[(p, base, s)])}
    sets = SETS + ([(SETS[1][0] + "、両方無効の組を除く", SETS[1][1] - both_inv), (SETS[0][0] + "、両方無効の組を除く", SETS[0][1] - both_inv)] if both_inv else [])
    for sname, S in sets:
        prs = sorted({p for (p, c, s) in R if c == cmb and p in S and (p, base, s) in R})
        if not prs:
            print(f"| {sname} | 0 | | | | | | | | |"); continue
        lr = {p: math.log((t(R[(p, cmb, 1)]) + SH) / (t(R[(p, base, 1)]) + SH)) for p in prs}
        v = list(lr.values()); m = sum(v) / len(v)
        se = (sum((x - m) ** 2 for x in v) / (len(v) - 1)) ** 0.5 / len(v) ** 0.5 if len(v) > 1 else float("nan")
        nb = sum(solved(R[(p, base, 1)]) for p in prs); nc = sum(solved(R[(p, cmb, 1)]) for p in prs)
        ob = [p for p in prs if solved(R[(p, base, 1)]) and not solved(R[(p, cmb, 1)])]
        oc = [p for p in prs if solved(R[(p, cmb, 1)]) and not solved(R[(p, base, 1)])]
        slow = sorted((p for p in prs if math.exp(lr[p]) > 1.25), key=lambda p: -lr[p])
        good = sorted((p for p in prs if math.exp(lr[p]) < 0.8), key=lambda p: lr[p])
        hi = math.exp(m + 1.96 * se)
        why = [w for w, c in (("区間上端", hi > 1.05), ("解けた数", nc < nb), ("遅れ", bool(slow))) if c]
        print(f"| {sname} | {len(prs)} | {math.exp(m):.3f} | [{math.exp(m - 1.96 * se):.3f}, {hi:.3f}] | {nb} → {nc} | {len(ob)} | {len(oc)} | {len(slow)} | {len(good)} | "
              + ("候補" if not why and not bad else "外れ (" + "・".join(why + (["食い違い"] if bad else [])) + ")") + " |")
        detail[sname] = (lr, ob, oc, slow, good)
    if both_inv:
        print(f"\n両方とも記録が無効な組 {len(both_inv)}: " + "; ".join(f"{p} (combo {st(R[(p, cmb, 1)])} / 既定 {st(R[(p, base, 1)])})" for p in sorted(both_inv)))
    if "240 問全部" in detail:
        lr, ob, oc, slow, good = detail["240 問全部"]
        tag = lambda p: "M" if p in SETS[0][1] else "残り"
        f = lambda p: f"{p} ({tag(p)}) {math.exp(lr[p]):.2f}: combo {desc(R[(p, cmb, 1)])} / 既定 {desc(R[(p, base, 1)])}"
        print(f"\n既定だけ解けた {len(ob)}: " + "; ".join(f(p) for p in ob))
        print(f"combo だけ解けた {len(oc)}: " + "; ".join(f(p) for p in oc))
        print(f"\n1.25 倍超の遅れ {len(slow)} 問 (240 問全部、比の大きい順):")
        for p in slow: print("  ", f(p))
        print(f"\n効いた (0.8 倍未満) {len(good)} 問 (240 問全部、比の小さい順):")
        for p in good: print("  ", f(p))
