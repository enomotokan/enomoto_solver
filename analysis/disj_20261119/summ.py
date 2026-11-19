"""集計 (python summ.py res_P.jsonl [ref_M_vsscip_def.json])。1 行 = (問題, 種)。
- 根のループ = `CUTSTOP` の行のうち bg (裏のカットの写し) でないもの。first = 最初のループ (再開の前)、last = 最後のループ。
- 閉じたギャップ = (根の下界 - 最初の LP) / (参照値 - 最初の LP)。根の下界は first なら最初のループの終わりの LP の値、final なら
  全ループの終わりの値の最大 (再開の後を含む)。参照値は scripts/miplib2017-v31.solu の =opt=/=best= (無ければその回の暫定解)。
  値はどれも内部の向き (最小化) にそろえる (solu・暫定解に sense を掛ける)。残りのギャップ = 1 - 閉じたギャップ (final)。
- t_root = 主のループの t_loop の和、t_all = 求解全体 (result.time)。
- 2 つ目の引数 (M の根だけの計測): 求解全体の状態・時間・ノードを vsscip_20261117 の def (0.1.9、600 秒、種 1) から付ける (参考)。"""
import json, sys, math
from pathlib import Path
D = Path(__file__).resolve().parent
solu = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    f = l.split()
    if len(f) >= 3 and f[0] in ("=opt=", "=best="):
        solu[f[1]] = float(f[2])
ref_full = json.load(open(D / sys.argv[2])) if len(sys.argv) > 2 else {}
rows = []
for l in open(D / sys.argv[1]):
    r = json.loads(l)
    res = r["result"]
    main = [s for s in r["cutstop"] if not s["bg"]]
    row = {"problem": r["problem"], "seed": r["seed"], "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"),
           "nloops": len(main), "nbg": len(r["cutstop"]) - len(main)}
    if ref_full:
        f = ref_full.get(r["problem"], {})
        row.update(full_status=f.get("status"), full_time=f.get("time"), full_nodes=f.get("nodes"))
    if main:
        s0, sl = main[0], main[-1]
        sense = s0["sense"]
        ref = solu.get(r["problem"])
        src = "solu"
        if ref is None and res.get("objective") is not None:
            ref, src = res["objective"], "run"
        lp0 = s0["obj0"]
        okl = [s["obj1"] for s in main if s["obj1"] is not None and s.get("lp_ok") is not False]
        rb_first = s0["obj1"]
        rb_final = max(okl) if okl else None
        row.update(reason_first=s0["reason"], reason_last=sl["reason"], reasons=[s["reason"] for s in main], restarts=sl["restarts"],
                   rounds_first=s0["rounds"], phase2_first=s0["phase2"], nfrac_first=s0["nfrac"], nint_first=s0["nint"], fsum_first=s0["fsum"],
                   nfrac_last=sl["nfrac"], nint_last=sl["nint"], t_root=round(sum(s["t_loop"] for s in main), 3), t_first_end=s0["t"], ref_src=src,
                   lp0=lp0, rb_first=rb_first, rb_final=rb_final, inc_first=s0["inc"])
        if ref is not None and lp0 is not None:
            refi = sense * ref
            den = refi - lp0
            row["ref"] = refi
            if abs(den) > 1e-6 * max(1.0, abs(refi)):
                cl = lambda v: None if v is None else max(-1.0, min(1.0, (v - lp0) / den))
                row["closed_first"], row["closed_final"] = cl(rb_first), cl(rb_final)
            else:
                row["closed_first"] = row["closed_final"] = None  # 根の LP がすでに参照値 (ギャップ 0)
            if rb_final is not None:
                row["relgap_final"] = abs(refi - rb_final) / max(abs(refi), abs(rb_final), 1e-9)
    rows.append(row)
json.dump(rows, open(D / (Path(sys.argv[1]).stem + "_rows.json"), "w"), indent=0)
from collections import Counter
print(f"{len(rows)} 回")
print("最初のループの止まり方:", Counter(r.get("reason_first") for r in rows).most_common())
print("最後のループの止まり方:", Counter(r.get("reason_last") for r in rows).most_common())
print()
hdr = f"{'problem':24s} sd {'status':10s} {'t_all':>6s} {'t_root':>6s} {'nodes':>7s} reasons                      {'cl1':>6s} {'clF':>6s} {'rem':>6s} {'nfr/nint':>10s}"
if ref_full:
    hdr += "  full(0.1.9) status time"
print(hdr)
fm = lambda v: "     -" if v is None else f"{v:6.3f}"
for r in rows:
    rem = None if r.get("closed_final") is None else 1 - r["closed_final"]
    line = (f"{r['problem']:24s} {r['seed']:2d} {str(r['status'])[:10]:10s} {r['time'] or 0:6.1f} {r.get('t_root', 0):6.2f} {r['nodes'] or 0:7d} "
            f"{','.join(r.get('reasons', []))[:28]:28s} {fm(r.get('closed_first'))} {fm(r.get('closed_final'))} {fm(rem)} {str(r.get('nfrac_first'))+'/'+str(r.get('nint_first')):>10s}")
    if ref_full:
        line += f"  {str(r.get('full_status'))[:10]:10s} {r.get('full_time') or 0:6.1f}"
    print(line)
