"""count.jsonl を表にする。使い方: python summ_count.py count.jsonl"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
import gprobs
recs = {}
for l in open(sys.argv[1]):
    r = json.loads(l)
    recs[r["problem"]] = r
CL = ["uniq", "red", "infeas", "force"]
for s, ps in gprobs.SETS.items():
    rs = [recs[p] for p in ps if p in recs]
    fin = [r for r in rs if r["final"]]
    tot = lambda key, sel: [sum(sel(r)[key][k] for r in fin) for k in range(4)]
    print(f"## {s}: {len(rs)} 問 (前処理の最後まで数えた {len(fin)} 問、時間切れ {sum(r['timeout'] for r in rs)} 問)")
    print(f"- 最後の問題: 整数列だけの不等式 {sum(r['final']['ineq'] for r in fin)} 本 (g>1 {sum(r['final']['g_gt1'] for r in fin)} 本、それを持つ問題 {sum(r['final']['g_gt1'] > 0 for r in fin)} 問)")
    for key, name in (("raw", "丸める前"), ("rnd", "丸めた後"), ("only_rnd", "丸めで初めて")):
        t = tot(key, lambda r: r["final"])
        hit = {c: [r["problem"] for r in fin if r["final"][key][k]] for k, c in enumerate(CL)}
        print(f"  - {name}: " + ", ".join(f"{c} {t[k]} 本 ({len(hit[c])} 問{': ' + ' '.join(hit[c][:6]) if hit[c] else ''})" for k, c in enumerate(CL)))
    print(f"  - 等式で右辺が g の倍数でない: {sum(r['final']['eq_infeas'] for r in fin)} 本")
    # 前処理の各呼び出しの丸めの直前
    calls = [(r["problem"], c) for r in rs for c in r["calls"]]
    for key, name in (("raw", "丸める前"), ("only_rnd", "丸めで初めて")):
        t = [sum(c[key][k] for _, c in calls) for k in range(4)]
        hit = sorted({p for p, c in calls if any(c[key])})
        print(f"- 前処理の呼び出しごとの丸めの直前 ({len(calls)} 回) {name}: " + ", ".join(f"{c} {t[k]}" for k, c in enumerate(CL)) + f" ({len(hit)} 問: {' '.join(hit[:10])})")
    gt = sum(c["g_gt1"] for _, c in calls); iq = sum(c["ineq"] for _, c in calls)
    print(f"- 前処理の呼び出しごとの丸めの直前: 整数列だけの不等式 {iq} 本、うち g>1 (丸めが係数を割る) {gt} 本 ({len({p for p, c in calls if c['g_gt1']})} 問)")
    imp = [r for r in rs if r.get("final_impl")]
    if imp:
        ti = lambda key: [sum(r["final_impl"][key][k] for r in imp) for k in range(4)]
        add = sum(r["final_impl"]["ineq"] - r["final"]["ineq"] for r in imp if r["final"])
        hit = sorted({r["problem"] for r in imp if any(r["final_impl"]["raw"]) or any(r["final_impl"]["only_rnd"]) or r["final_impl"]["eq_infeas"]})
        print(f"- 暗黙の整数列も整数として数えた最後の問題 ({len(imp)} 問で印が増える): 不等式 +{add} 本、丸める前 {ti('raw')}、丸めで初めて {ti('only_rnd')}、等式の実行不能 {sum(r['final_impl']['eq_infeas'] for r in imp)} ({' '.join(hit[:10])})")
    to = [r["problem"] for r in rs if not r["final"]]
    if to:
        print(f"- 最後まで数えられなかった問題: {' '.join(to)}")
    print()
