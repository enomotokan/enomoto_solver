"""根の強分岐の候補の集計 (python sbana.py res_SB_P.jsonl [res_SB_M.jsonl ...])。
- 1 回 = (問題, 種)。根の分枝の選択は `Resolve` (強分岐で列を固定して解き直す) で何回か呼ばれうるので、`SBCAND` を直後の `SBEND` で
  呼び出しごとに区切り、**分枝した呼び出し (action branch) の候補** を使う (無ければ最後の呼び出し)。z はその呼び出しの根の LP の値。
- 候補 j の m_j = min(下の子, 上の子) (片側が打ち切りならもう片方、子の値は反復上限なら途中の値 = 下界)。
- 残りギャップ G = 参照値 - z (参照値は scripts/miplib2017-v31.solu、内部の向き)。
- (a) A = (max_j m_j - z) / G: 1 つの離接のカットで根の下界が上がる幅の上限 (の、評価した候補の中での最大)。
- (b) 選ばれなかった候補で m_j > z (1e-6 max(1,|z|) を超えて) の数と、その (m_j - z) / G の最大・中央値。
- 参考に、分枝した列の (m - z) / G、Σother = 選ばれなかった候補の (m_j - z) / G の和、inex = 子の LP が反復上限で途中の値
  (下界なので本当の子の値はもっと高いかもしれない) の候補の数。"""
import json, sys, statistics
from pathlib import Path
D = Path(__file__).resolve().parent
solu = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] in ("=opt=", "=best="):
        solu[f[1]] = float(f[2])
STALL = {"stall", "intobj_stall", "phase2_stall"}
out = []
for fn in sys.argv[1:]:
    for l in open(D / fn):
        r = json.loads(l)
        main = [s for s in r["cutstop"] if not s["bg"]]
        sense = main[0]["sense"] if main else 1
        # 候補と呼び出しの終わりは詳細ログの行の順に読む (jsonl の配列は別々なので)
        import gzip
        lg = D / "logs" / f"sb_{r['problem']}_{r['tl']}_s{r['seed']}{'_root' if r['root'] else ''}.err.gz"
        if lg.exists():
            r["sbcand"], r["sbend"] = [], []
            for li, ln in enumerate(gzip.open(lg, "rt")):
                for tag, key in (("SBCAND ", "sbcand"), ("SBEND ", "sbend")):
                    if ln.startswith(tag):
                        r[key].append(dict(json.loads(ln[len(tag):]), _i=li))
        calls, cur = [], []
        # SBCAND と SBEND は出た順に並んでいる (同じスレッド)。jsonl では別の配列なので t で並べ直す
        ev = [("c", c) for c in r["sbcand"]] + [("e", e) for e in r["sbend"]]
        ev.sort(key=lambda x: x[1].get("_i", x[1]["t"]))  # ログの行の順 (古い行は時刻、同じ時刻なら候補が先)
        for k, x in ev:
            if k == "c":
                cur.append(x)
            else:
                calls.append((x, cur))
                cur = []
        row = {"problem": r["problem"], "seed": r["seed"], "src": fn, "status": r["result"].get("status"),
               "reason_first": main[0]["reason"] if main else None, "ncalls": len(calls)}
        if not calls:
            out.append(row)
            continue
        br = [c for c in calls if c[0]["action"] == "branch"]
        e, cands = br[0] if br else calls[-1]
        z = e["z"]
        ref = solu.get(r["problem"])
        if ref is None and r["result"].get("objective") is not None:
            ref = r["result"]["objective"]
        G = sense * ref - z if ref is not None else None
        tol = 1e-6 * max(1.0, abs(z))
        def m(c):
            vals = [v for v in (c["down"], c["up"]) if v is not None]
            return min(vals) if vals else float("inf")
        ms = [(c["col"], m(c), c) for c in cands]
        row.update(action=e["action"], nfrac=e["nfrac"], neval=len(cands), z=z, G=G, chosen=e["col"],
                   ncut1=sum((c["cut_down"] or c["cut_up"]) for c in cands),
                   ninexact=sum((not c["cut_down"] and not c["exact_down"]) or (not c["cut_up"] and not c["exact_up"]) for c in cands))
        if G is not None and G > tol and ms:
            gmax = max(min(mm, sense * ref) for _, mm, _ in ms) - z
            row["A"] = gmax / G
            other = [min(mm, sense * ref) - z for col, mm, _ in ms if col != e["col"]]
            up = [g for g in other if g > tol]
            row["b_n"] = len(up)
            row["b_nother"] = len(other)
            row["b_max"] = max(up) / G if up else 0.0
            row["b_med"] = statistics.median(up) / G if up else 0.0
            ch = [min(mm, sense * ref) - z for col, mm, _ in ms if col == e["col"]]
            row["chosen_gain"] = ch[0] / G if ch else None
            row["sum_other"] = sum(up) / G
        out.append(row)
json.dump(out, open(D / "sb_rows.json", "w"), indent=0)
print(f"{'problem':22s} sd {'stop':12s} {'act':7s} {'nfrac':>5s} {'eval':>4s} {'A':>7s} {'chosen':>7s} {'b_n/oth':>8s} {'b_max':>7s} {'b_med':>7s} {'Σother':>7s} {'inex':>4s}")
f = lambda v: "      -" if v is None else f"{v:7.4f}"
for r in out:
    print(f"{r['problem']:22s} {r['seed']:2d} {str(r.get('reason_first'))[:12]:12s} {str(r.get('action'))[:7]:7s} {r.get('nfrac', 0):5d} {r.get('neval', 0):4d} {f(r.get('A'))} {f(r.get('chosen_gain'))} "
          f"{str(r.get('b_n', '-')) + '/' + str(r.get('b_nother', '-')):>8s} {f(r.get('b_max'))} {f(r.get('b_med'))} {f(r.get('sum_other'))} {r.get('ninexact', 0):4d}")
