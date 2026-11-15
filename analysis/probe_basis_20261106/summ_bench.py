"""heavy300.jsonl・bench58.jsonl の表。使い方: python summ_bench.py heavy300|bench58"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
name = sys.argv[1]
rows = [json.loads(l) for l in open(D / f"{name}.jsonl")]
if not name.startswith("bench"):
    print("| 問題 | 設定 | 回 | 根の LP 秒 (反復) | 根の終わり 秒 | 根の下界 (カットの後) | 300 秒の下界 | 解 | 試作のログ |")
    print("|---|---|---:|---|---:|---|---|---|---|")
    for r in rows:
        rl, res = r["rootlp"], r["result"]
        lp = f"{rl['t']:.1f} ({rl['it']})" if "t" in rl else f"時間切れ ({rl.get('stopped')} 秒)"
        g = [n.replace("MIP: probe basis guard: ", "") for n in r["notes"] if "guard" in n]
        b = res.get("bound"); o = res.get("objective")
        print(f"| {r['problem']} | {r['cfg']} | {r['rep']} | {lp} | {rl.get('root_end', '-')} | {rl.get('root_bound', '-')} | {'-' if b is None or (isinstance(b, float) and math.isinf(b)) else f'{b:.6g}'} | {'-' if o is None else f'{o:.6g}'} | {'; '.join(g)[:120]} |")
else:
    by = defaultdict(dict)
    for r in rows:
        by[(r["cfg"], r["rep"])][r["problem"]] = r
    cfgs = sorted({k[0] for k in by}); reps = sorted({k[1] for k in by})
    print("| 回 | 設定 | 問題 | 解けた数 | 幾何平均 (秒、+1) | 試作が働いた問題 |")
    print("|---:|---|---:|---:|---:|---|")
    for rep in reps:
        common = set.intersection(*[set(by[(c, rep)]) for c in cfgs if (c, rep) in by])
        for c in cfgs:
            if (c, rep) not in by:
                continue
            d = by[(c, rep)]
            ok = [p for p in common if d[p]["result"].get("status") == "optimal"]
            sgm = math.exp(sum(math.log(min(d[p]["result"].get("time") or 60, 60) + 1) for p in common) / max(len(common), 1)) - 1
            hit = sorted(p for p in common if any(("(ENOMOTO_MIP_PROBE_BASIS_GUARD)" in n) or ("probe basis guard" in n and "hit the cap" in n) or ("root LP raced from the basis" in n) for n in d[p]["notes"]))
            print(f"| {rep} | {c} | {len(common)} | {len(ok)} | {sgm:.2f} | {', '.join(hit)} |")
    print()
    print("差の大きい問題 (どちらかが解けた問題で時間の比が 1.3 倍を超えるもの):")
    for rep in reps:
        for c2 in cfgs:
          if c2 == "def" or ("def", rep) not in by or (c2, rep) not in by:
            continue
          a, b = by[("def", rep)], by[(c2, rep)]
          for p in sorted(set(a) & set(b)):
            ta, tb = a[p]["result"].get("time") or 60, b[p]["result"].get("time") or 60
            sa, sb = a[p]["result"].get("status"), b[p]["result"].get("status")
            if (sa == "optimal" or sb == "optimal") and (max(ta, tb) + 1) / (min(ta, tb) + 1) > 1.3:
                print(f"- 回 {rep} {p}: def {sa} {ta:.1f} / {c2} {sb} {tb:.1f}")
