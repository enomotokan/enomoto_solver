"""full モード (`ENOMOTO_MIP_SBCAND_LOG=full`) の集計 (python sbfana.py res_SBF_P.jsonl [res_SBF_M.jsonl])。
根 (最後の再開の後、最初の分枝の選択の前) の LP の値 z で、小数の整数列を f(1-f) の大きい順に 200 本まで (60 秒まで)、両側の子の LP を
反復の上限なしで解いた値から:
- m_j = min(下の子, 上の子) (子が実行不能・打ち切り (null) ならもう片方。両方なら参照値)。値は参照値で頭打ちにする。
- G = 参照値 - z (参照値は miplib2017-v31.solu、内部の向き)。
- A = (max_j m_j - z) / G、n1・n10 = (m_j - z) / G が 1%・10% を超える候補の数。
- b: 分枝に選ばれた列 (`SBEND` の col) を除いた候補で m_j > z (1e-6 max(1,|z|) を超えて) の数と (m_j - z) / G の最大・中央値。
- z1 = 単純な 1 本の離接の上限を重ねた目安 Σ_j (m_j - z) / G (上限ではない。重ねたカットで届きうる量の粗い目安)。"""
import gzip, json, sys, statistics
from pathlib import Path
D = Path(__file__).resolve().parent
solu = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] in ("=opt=", "=best="):
        solu[f[1]] = float(f[2])
out = []
for fn in sys.argv[1:]:
    for l in open(D / fn):
        r = json.loads(l)
        main = [s for s in r["cutstop"] if not s["bg"]]
        sense = main[0]["sense"] if main else 1
        row = {"problem": r["problem"], "seed": r["seed"], "src": fn, "reason_first": main[0]["reason"] if main else None,
               "nfull": len(r["sbfull"])}
        ends = r["sbfullend"]
        if not ends or not r["sbfull"]:
            out.append(row)
            continue
        last_r = ends[-1]["restarts"]
        full = [x for x in r["sbfull"] if x["restarts"] == last_r]
        e = ends[-1]
        z = e["z"]
        ref = solu.get(r["problem"])
        if ref is None and r["result"].get("objective") is not None:
            ref = r["result"]["objective"]
        if ref is None:
            out.append(row)
            continue
        refi = sense * ref
        G = refi - z
        tol = 1e-6 * max(1.0, abs(z))
        chosen = next((x["col"] for x in r["sbend"] if x["restarts"] == last_r and x["action"] == "branch"), None)
        def m(x):
            vals = [v for v in (x["down"], x["up"]) if v is not None]
            return min(min(vals) if vals else refi, refi)
        gs = [(x["col"], m(x) - z) for x in full]
        row.update(nfrac=e["nfrac"], done=e["done"], stopped=e["stopped"], secs=e["secs"], G=G, z=z, chosen=chosen,
                   ninf=sum(x["down"] is None or x["up"] is None for x in full))
        if G > tol:
            row["A"] = max(g for _, g in gs) / G
            row["n1"] = sum(g > 0.01 * G for _, g in gs)
            row["n10"] = sum(g > 0.10 * G for _, g in gs)
            other = [g for c, g in gs if c != chosen]
            up = [g for g in other if g > tol]
            row.update(b_n=len(up), b_nother=len(other), b_max=max(up) / G if up else 0.0, b_med=statistics.median(up) / G if up else 0.0,
                       zsum=sum(max(g, 0) for _, g in gs) / G)
            ch = [g for c, g in gs if c == chosen]
            row["chosen_gain"] = ch[0] / G if ch else None
        out.append(row)
json.dump(out, open(D / ("sbf_rows_" + Path(sys.argv[1]).stem + ".json"), "w"), indent=0)
f = lambda v: "      -" if v is None else f"{v:7.4f}"
print(f"{'problem':22s} {'stop':12s} {'nfrac':>5s} {'done':>4s} {'secs':>6s} {'A':>7s} {'n1':>4s} {'n10':>4s} {'chosen':>7s} {'b_n/oth':>9s} {'b_max':>7s} {'b_med':>7s} {'Σ':>7s} inf")
for r in out:
    print(f"{r['problem']:22s} {str(r.get('reason_first'))[:12]:12s} {r.get('nfrac', 0):5d} {r.get('done', 0):4d} {r.get('secs', 0):6.1f} {f(r.get('A'))} {r.get('n1', '-'):>4} {r.get('n10', '-'):>4} {f(r.get('chosen_gain'))} "
          f"{str(r.get('b_n', '-')) + '/' + str(r.get('b_nother', '-')):>9s} {f(r.get('b_max'))} {f(r.get('b_med'))} {f(r.get('zsum'))} {r.get('ninf', 0)}")
