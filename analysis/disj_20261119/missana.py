"""強分岐の見逃しの切り分け (python missana.py res_SBF_P.jsonl res_SBF_M.jsonl)。full モードの回のログには、同じ根の既定の強分岐の
`SBCAND` (評価した候補と子の値、反復上限なら途中の値) と `SBEND` (分枝した列) も出ているので、同じ回の中で突き合わせる。
- g_full(j) = min(子) - z (full、反復の上限なし、参照値で頭打ち)、A = max g_full / G。
- 既定の強分岐は最初の呼び出し (full と同じ z) の候補 E と g_def(j) (同じ定義、子が途中の値なら低めに出る)。
- 見逃し = A >= 1% で、分枝した列 c の g_full(c) < A / 2。
- 原因 (g_full が最大の列 j* について): (a) j* が E にない (候補の絞り)、(b) j* は E にあるが g_def(j*) < g_full(j*) / 2
  (反復の上限で低く見積もった)、(c) j* は E にあり g_def も十分だが別の列が選ばれた (スコア)。
- 「良い列」 = g_full >= A / 2 の列について、同じ (a)(b)(c) の本数も数える。"""
import gzip, json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
solu = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] in ("=opt=", "=best="):
        solu[f[1]] = float(f[2])
rows = []
for fn in sys.argv[1:]:
    for l in open(D / fn):
        r = json.loads(l)
        p = r["problem"]
        lg = D / "logs" / f"sbf_{p}_{r['tl']}_s{r['seed']}_root.err.gz"
        ev = []
        for ln in gzip.open(lg, "rt"):
            for tag in ("SBFULL ", "SBFULLEND ", "SBCAND ", "SBEND "):
                if ln.startswith(tag):
                    ev.append((tag.strip(), json.loads(ln[len(tag):])))
        main = [s for s in r["cutstop"] if not s["bg"]]
        sense = main[0]["sense"] if main else 1
        ref = solu.get(p)
        if ref is None or not any(t == "SBFULLEND" for t, _ in ev):
            continue
        # 最後の SBFULLEND の後の最初の SBEND までが既定の強分岐の最初の呼び出し
        k = max(i for i, (t, _) in enumerate(ev) if t == "SBFULLEND")
        fend = ev[k][1]
        z = fend["z"]
        full = [x for t, x in ev[:k] if t == "SBFULL" and x["restarts"] == fend["restarts"]]
        cand, end = [], None
        for t, x in ev[k + 1:]:
            if t == "SBCAND":
                cand.append(x)
            elif t == "SBEND":
                end = x
                break
        if end is None or not full:
            continue
        refi = sense * ref
        G = refi - z
        if G <= 1e-6 * max(1.0, abs(z)):
            continue
        def g(x):
            vals = [v for v in (x["down"], x["up"]) if v is not None]
            return min(min(vals) if vals else refi, refi) - x["z"]
        gf = {x["col"]: g(x) for x in full}
        gd = {x["col"]: (g(x), (x["cut_down"] or x["exact_down"]) and (x["cut_up"] or x["exact_up"])) for x in cand}
        # 最初の呼び出しが解き直し (resolve) なら、分枝した列は後の呼び出し。分枝した列は最初の branch の SBEND
        br = next((x for t, x in ev[k + 1:] if t == "SBEND" and x["action"] == "branch"), None)
        c = br["col"] if br else None
        A = max(gf.values()) / G
        jstar = max(gf, key=gf.get)
        gc = gf.get(c)
        good = [j for j, v in gf.items() if v >= 0.5 * A * G and v > 0]
        def cause(j):
            if j not in gd:
                return "a"
            v, exact = gd[j]
            return "b" if v < 0.5 * gf[j] else "c"
        miss = A >= 0.01 and (gc is None or gc < 0.5 * A * G)
        row = {"problem": p, "seed": r["seed"], "src": fn, "A": A, "chosen": c, "chosen_sb": br["sb"] if br else None,
               "g_chosen": None if gc is None else gc / G, "chosen_in_full": c in gf, "nfrac": fend["nfrac"], "nfull": len(full),
               "neval": len(cand), "first_action": end["action"], "miss": miss, "jstar": jstar,
               "cause": cause(jstar) if miss else None, "good": len(good),
               "good_a": sum(cause(j) == "a" for j in good), "good_b": sum(cause(j) == "b" for j in good), "good_c": sum(cause(j) == "c" for j in good),
               "ninexact": sum(not e for _, e in gd.values())}
        if jstar in gd:
            row["g_def_jstar"] = gd[jstar][0] / G
        rows.append(row)
json.dump(rows, open(D / "miss_rows.json", "w"), indent=0)
f = lambda v: "      -" if v is None else f"{v:7.4f}"
print(f"{'problem':22s} {'A':>7s} {'g(c)':>7s} c_sb  miss cause {'g_def*':>7s} good a/b/c   eval/nfrac inex first")
for r in rows:
    print(f"{r['problem']:22s} {f(r['A'])} {f(r['g_chosen'])} {str(r['chosen_sb'])[:5]:5s} {str(r['miss'])[:5]:5s} {str(r['cause']):5s} {f(r.get('g_def_jstar'))} "
          f"{r['good']:4d} {r['good_a']}/{r['good_b']}/{r['good_c']:<4d} {r['neval']:4d}/{r['nfrac']:<5d} {r['ninexact']:4d} {r['first_action']}")
from collections import Counter
ms = [r for r in rows if r["miss"]]
print(f"\nA >= 1% の回: {sum(r['A'] >= 0.01 for r in rows)}、見逃し: {len(ms)}、原因 (最良の列): {Counter(r['cause'] for r in ms)}")
print(f"見逃しの回の良い列の合計: a {sum(r['good_a'] for r in ms)}・b {sum(r['good_b'] for r in ms)}・c {sum(r['good_c'] for r in ms)}")
print(f"見逃しのうち分枝した列が強分岐でなく擬似費用で選ばれた (sb false): {sum(r['chosen_sb'] is False for r in ms)}")
