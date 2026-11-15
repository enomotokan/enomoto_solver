"""count.jsonl の集計 (集合ごとの表と、何か出た問題の一覧)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
import hprobs
R = {json.loads(l)["problem"]: json.loads(l) for l in (D / "count.jsonl").open()}
def hit(r): return r.get("fixes") or r.get("congs") or r.get("infeasible")
print("| 集合 | 問題 | 行の出ない問題 | 整数列だけの等式がある | 諦めた | 実行不能 | 固定が出る | 合同式が出る (うち上下限が詰まる) | 一般整数の合同式 |")
print("|---|---|---|---|---|---|---|---|---|")
for s, ps in hprobs.SETS.items():
    rs = [R[p] for p in ps if p in R]
    nol = [r for r in rs if r.get("eq") is None]
    has = [r for r in rs if (r.get("int_eq") or 0) > 0]
    gu = [r for r in rs if (r.get("gave_up") or 0) > 0]
    inf = [r for r in rs if r.get("infeasible")]
    fx = [r for r in rs if r.get("fixes")]
    cg = [r for r in rs if r.get("congs")]
    cgt = [r for r in rs if r.get("congs_tighten")]
    cgg = [r for r in rs if r.get("congs_gen")]
    print(f"| {s} | {len(rs)} | {len(nol)} | {len(has)} | {len(gu)} | {len(inf)} | {len(fx)} | {len(cg)} ({len(cgt)}) | {len(cgg)} |")
print()
print("| 問題 | 集合 | 整数の等式 | 列 | 階数 | 固定 | 合同式 (一般整数, 詰まる, 最大の法) | 実行不能 | 諦めた (溢れ/手間/大きさ) | 秒 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for p, r in R.items():
    if not (hit(r) or r.get("gave_up")):
        continue
    sets = ",".join(s for s, ps in hprobs.SETS.items() if p in ps)
    print(f"| {p} | {sets} | {r['int_eq']} | {r['cols']} | {r['rank']} | {r['fixes']} | {r['congs']} ({r['congs_gen']}, {r['congs_tighten']}, {r['max_mod']}) | {'**実行不能**' if r['infeasible'] else ''} | {r['gave_up']} ({'/'.join(map(str, r['gave_why']))}) | {r['secs']} |")
print()
print("行の出ない問題 (前処理で実行不能・前処理が何も減らさない・前処理の途中で止まった):", " ".join(p for p, r in R.items() if r.get("eq") is None))
print("neos859080:", R.get("neos859080"))
