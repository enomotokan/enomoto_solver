"""並列の効きの内訳の集計: python summ.py [P|M|PM] > summ.txt。res_*.jsonl と logs/*.log.gz から問題ごと・スレッド数ごとに次を出す
(種ごとに出し、最後に問題をまとめた表と改善案の見積り)。
- 時間 T (時間切れは上限)、速さ T1/Tk、根の段の秒 R (外側の時計で最初に木に入った時刻、木に入らなければ T) と R/T (Amdahl の上限は T/R)、木の段 T−R
- 節点 (全スレッドの合計) の比 Nk/N1 (探索の水増し)、木の段の 1 節点あたりのスレッド秒 (k × (T−R) / N) の比
- ノードの最初の LP の平均反復 (潜り・自分の待ち行列・共有の待ち行列、`TREEPAR`)、スレッドの遊び時間 (待ち + 終わりの待ち) の合計と木の段のスレッド秒に対する割合
- 最終の暫定解を見つけた時刻 (外側の時計、近似)、再スタートの回数 (全部・木の中)
見積り:
(1) 根の段で遊ぶスレッドを使う: 根の段が k 倍速くなった場合の上限 T − R (1 − 1/k)。
(2) 再スタートの判定をそろえる: 木の中の再スタートの回数が 1 スレッドと k スレッドで違う問題を挙げる。
(3) 受け渡すノードに基底を付ける: 待ち行列・共有の待ち行列から取ったノードの最初の LP を、潜りの LP と同じ反復・秒にできた場合に浮くスレッド秒 / k。
(4) 競争的な使い方: 種による揺れ (k スレッドの 2 種の max/min)。"""
import gzip, json, glob, math, re, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
SETS = {"P": ["P"], "M": ["M"], "PM": ["P", "M"]}[sys.argv[1] if len(sys.argv) > 1 else "PM"]
rows = [json.loads(l) for S in SETS for f in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl"))) for l in open(f) if l.strip()]
def solved(r): return (r["result"].get("status") or "").lower() in ("optimal", "infeasible")
def parse(S, r):
    p, th, s = r["problem"], r["threads"], r["seed"]
    f = D / "logs" / f"{S}_{p}_t{th}_s{s}.log.gz"
    T = min(r["result"].get("time") or r["tl"], r["tl"]) if solved(r) else r["tl"]
    out = dict(T=T, R=T, rst=0, trst=0, inc_t=None, lp={"dive": [0, 0.0, 0.0], "queue": [0, 0.0, 0.0], "pool": [0, 0.0, 0.0]}, idle=0.0, tail=0.0, lns=0.0)
    if not f.exists(): return out
    lines = gzip.decompress(f.read_bytes()).decode(errors="replace").splitlines()
    first_tree = None; pend = []; final_z = r["result"].get("objective")
    for l in lines:
        m = re.match(r"TRSTAT: tree start outer ([0-9.]+) solve ([0-9.]+)", l)
        if m:
            o, sv = float(m.group(1)), float(m.group(2))
            if first_tree is None: first_tree = o
            for (z, t) in pend:
                if final_z is not None and abs(z - final_z) <= 1e-6 * max(1, abs(final_z)) and out["inc_t"] is None: out["inc_t"] = t + (o - sv)
            pend = []
            continue
        m = re.search(r"new incumbent ([-0-9.e+]+) at node \d+ \(([0-9.]+)s\)", l)
        if m: pend.append((float(m.group(1)), float(m.group(2))))
        if re.search(r"^MIP: restart [0-9]+ \(", l): out["rst"] += 1
        if re.search(r"TREE restart|tree restart:", l): out["trst"] += 1
        if l.startswith("TREEPAR: balance") and "| tid" in l:
            t = l.split("| tid", 1)[1]
            for key in ("idle", "tail", "lns"):
                mm = re.search(key + r" ([0-9.]+)s", t)
                if mm: out[key] += float(mm.group(1))
            for kind in ("dive", "queue", "pool"):
                mm = re.search(kind + r" (\d+) ([0-9.]+)it ([0-9.]+)ms", t)
                if mm:
                    n, it, ms = int(mm.group(1)), float(mm.group(2)), float(mm.group(3))
                    a = out["lp"][kind]; a[0] += n; a[1] += n * it; a[2] += n * ms / 1000
    for (z, t) in pend:  # 最後の求解の木に入る前の解 (時計のずれは近似で 0 とみなせないので、木に入っていなければ T から引けない)
        if final_z is not None and abs(z - final_z) <= 1e-6 * max(1, abs(final_z)) and out["inc_t"] is None: out["inc_t"] = t
    if first_tree is not None: out["R"] = min(first_tree, T)
    return out
recs = {}
for f in [ff for S in SETS for ff in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl")))]:
    S = Path(f).name.split("_")[1]
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l); m = parse(S, r); m["N"] = r["result"].get("nodes") or 0; m["ok"] = solved(r); m["tl"] = r["tl"]
        recs[(r["problem"], r["threads"], r["seed"])] = m
probs = sorted({k[0] for k in recs}); ths = sorted({k[1] for k in recs}); seeds = sorted({k[2] for k in recs})
print(f"{'+'.join(SETS)}: 回 {len(recs)}、問題 {len(probs)}、スレッド {ths}、種 {seeds}、crash {sum(1 for r in rows if (r['result'].get('status') or '').startswith('crash'))}")
def lpstat(m, kind):
    n, it, sec = m["lp"][kind]; return (n, it / n if n else 0.0, 1000 * sec / n if n else 0.0)
print("\n## 回ごと (T 秒・R 根の段の秒・N 節点・木の 1 節点あたりのスレッド ms・最初の LP の平均反復 潜り/待ち行列/共有・遊び%・最終解の時刻・再スタート (木の中))")
for p in probs:
    for s in seeds:
        for th in ths:
            m = recs.get((p, th, s))
            if not m: continue
            tree = max(m["T"] - m["R"], 0.0); per = 1000 * th * tree / m["N"] if m["N"] else float("nan")
            d, q, pl = lpstat(m, "dive"), lpstat(m, "queue"), lpstat(m, "pool")
            idle = 100 * (m["idle"] + m["tail"]) / max(th * tree, 1e-9) if th > 1 else 0.0
            print(f"{p:20s} s{s} t{th}: T {m['T']:6.1f}{'' if m['ok'] else '*'} R {m['R']:6.1f} ({100*m['R']/max(m['T'],1e-9):3.0f}%) N {m['N']:8d} {per:7.2f}ms "
                  f"LP {d[1]:5.1f}/{q[1]:6.1f}/{pl[1]:6.1f} idle {idle:4.1f}% inc {('%.1f' % m['inc_t']) if m['inc_t'] is not None else '-':>6} rst {m['rst']}({m['trst']})")
print("\n## 問題ごと (2 種の幾何平均。速さ = T1/Tk、水増し = Nk/N1、1 節点の比 = 1 節点あたりのスレッド秒の比 k/1、根の割合 R/Tk)")
g = lambda v: math.exp(sum(math.log(max(x, 1e-9)) for x in v) / len(v)) if v else float("nan")
est = collections.defaultdict(list)
for p in probs:
    for th in ths:
        if th == 1: continue
        sp, inf, pn, rs, e1, e3, var = [], [], [], [], [], [], []
        for s in seeds:
            a, b = recs.get((p, 1, s)), recs.get((p, th, s))
            if not a or not b: continue
            sp.append(a["T"] / b["T"]); rs.append(b["R"] / max(b["T"], 1e-9))
            if a["N"] and b["N"]:
                inf.append(b["N"] / a["N"])
                ta, tb = max(a["T"] - a["R"], 1e-3), max(b["T"] - b["R"], 1e-3)
                pn.append((th * tb / b["N"]) / (ta / a["N"]))
            e1.append(b["T"] - b["R"] * (1 - 1 / th))
            d = lpstat(b, "dive")
            extra = sum(max(lpstat(b, kk)[2] - d[2], 0) * lpstat(b, kk)[0] / 1000 for kk in ("queue", "pool"))
            e3.append(max(b["T"] - extra / th, 1e-3))
        Tk = [recs[(p, th, s)]["T"] for s in seeds if (p, th, s) in recs]
        tr = {s: (recs.get((p, 1, s), {}).get("trst"), recs.get((p, th, s), {}).get("trst")) for s in seeds}
        if not sp: continue
        print(f"{p:20s} t{th}: 速さ {g(sp):5.2f} 水増し {g(inf):5.2f} 1 節点の比 {g(pn):5.2f} 根の割合 {100*sum(rs)/len(rs):3.0f}% "
              f"(1) {g(Tk)/g(e1):4.2f}倍 (3) {g(Tk)/g(e3):4.2f}倍 (4) 揺れ {max(Tk)/min(Tk):4.2f} 木の中の再スタート (1 本, {th} 本) {tr}")
        est[th].append((p, g(Tk), g(e1), g(e3), max(Tk) / min(Tk), tr))
print("\n## 改善案の見積り (k スレッドの時間の幾何平均の、案を当てた上限との比。1 より大きいほど効く見込み)")
for th, L in est.items():
    if not L: continue
    G = lambda i: g([x[i] for x in L])
    mism = [x[0] for x in L if any(a != b for a, b in x[5].values())]
    print(f"t{th}: (1) 根の段を {th} 倍に {G(1)/G(2):.3f} 倍、(3) 受け渡すノードの LP を潜り並みに {G(1)/G(3):.3f} 倍、(4) 2 種の揺れの幾何平均 {G(4):.2f}、"
          f"(2) 木の中の再スタートの回数が 1 本と違う問題 {len(mism)}: {mism}")
