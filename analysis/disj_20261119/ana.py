"""停滞の数え上げ (python ana.py res_P_rows.json)。停滞 = 最初のループの理由が stall・intobj_stall・phase2_stall・robust_ext_stall。
問題ごとに種をまとめる (2 種とも停滞なら「2/2」)。残りギャップ = 1 - 閉じたギャップ (final)。"""
import json, sys
from collections import defaultdict
STALL = {"stall", "intobj_stall", "phase2_stall", "robust_ext_stall"}
rows = json.load(open(sys.argv[1]))
# M (根だけ): 求解全体の状態・時間・ノードは vsscip_20261117 の def (0.1.9、600 秒、種 1、参考) に置き換える
FULL = any("full_status" in r for r in rows)
TL = 600 if FULL else 60
for r in rows:
    if FULL:
        r["root_time"] = r["time"]
        r["status"], r["time"], r["nodes"] = r.get("full_status"), r.get("full_time") or TL, r.get("full_nodes")
by = defaultdict(list)
for r in rows:
    by[r["problem"]].append(r)
def isst(r, which="reason_first"):
    return r.get(which) in STALL
n = len(rows)
st = [r for r in rows if isst(r)]
st_any = [r for r in rows if any(x in STALL for x in r.get("reasons", []))]
print(f"回: {n}、最初のループが停滞 {len(st)}、どれかのループが停滞 {len(st_any)}")
probs = list(by)
pst = [p for p in probs if any(isst(r) for r in by[p])]
pst2 = [p for p in probs if all(isst(r) for r in by[p])]
print(f"問題: {len(probs)}、少なくとも 1 種で最初のループが停滞 {len(pst)}、全種で停滞 {len(pst2)}")
def rem(r):
    c = r.get("closed_final")
    return None if c is None else 1 - c
big = [p for p in pst if any(isst(r) and (rem(r) or 0) > 0.10 for r in by[p])]
print(f"停滞かつ残りギャップ > 10% (少なくとも 1 種): {len(big)}")
print(f"\n{'problem':22s} {'理由(種1/種2)':34s} {'rem':>11s} {'nfrac/nint':>12s} {'t_root/t_all':>16s} {'状態':20s} nodes")
for p in sorted(big, key=lambda p: -max(rem(r) or 0 for r in by[p])):
    rs = by[p]
    print(f"{p:22s} {' / '.join(','.join(r.get('reasons', [])) for r in rs)[:34]:34s} "
          f"{' '.join(f'{rem(r):.2f}' if rem(r) is not None else '-' for r in rs):>11s} "
          f"{' '.join(str(r.get('nfrac_first')) for r in rs) + ' /' + str(rs[0].get('nint_first')):>12s} "
          f"{' '.join(f'{r.get(chr(116)+"_root",0):.1f}/{r[chr(116)+"ime"]:.0f}' for r in rs):>16s} {' '.join(str(r['status'])[:9] for r in rs):20s} {' '.join(str(r['nodes']) for r in rs)}")
small = [p for p in pst if p not in big]
print("\n停滞・残りギャップ <= 10% (またはギャップ 0):", ", ".join(f"{p}({' '.join(f'{rem(r):.2f}' if rem(r) is not None else '-' for r in by[p])})" for p in small))
# 根の時間の割合
fr = [r.get("t_root", 0) / r["time"] for r in rows if r.get("time")]
fr.sort()
import statistics
tot_root = sum(r.get("t_root", 0) for r in rows); tot_all = sum(r["time"] or 0 for r in rows)
print(f"\n根のループの時間 / 求解全体: 中央値 {statistics.median(fr):.3f}、平均 {statistics.mean(fr):.3f}、合計の比 {tot_root:.0f}/{tot_all:.0f} = {tot_root/tot_all:.3f}")
for lab, sel in (("停滞", [r for r in rows if isst(r)]), ("停滞でない", [r for r in rows if not isst(r)])):
    f = [r.get("t_root", 0) / r["time"] for r in sel if r.get("time")]
    print(f"  {lab}: {len(sel)} 回、割合の中央値 {statistics.median(f):.3f}、合計の比 {sum(r.get('t_root',0) for r in sel)/sum(r['time'] for r in sel):.3f}")
# 解けない・遅い (>= 10 秒) と停滞の重なり
slow = [p for p in probs if any(r["status"] != "optimal" or (r["time"] or 0) >= TL / 6 for r in by[p])]
print(f"\n解けない ({TL} 秒で時間切れ) か {TL // 6} 秒以上かかる問題: {len(slow)}")
for p in slow:
    rs = by[p]
    print(f"  {p:22s} {'停滞' if any(isst(r) for r in rs) else '    '} 理由 {' / '.join(','.join(r.get('reasons', [])) for r in rs)[:40]:40s} rem {' '.join(f'{rem(r):.2f}' if rem(r) is not None else '-' for r in rs)} 状態 {' '.join(f'{str(r['status'])[:4]}{r['time']:.0f}' for r in rs)} t_root {' '.join(f'{r.get('t_root',0):.1f}' for r in rs)}")
# 重なりのまとめ (問題単位、どれかの種で)
uns = [p for p in probs if any(r["status"] != "optimal" for r in by[p])]
slw = [p for p in probs if p not in uns and any((r["time"] or 0) >= TL / 6 for r in by[p])]
for lab, ps in ((f"解けない ({TL} 秒)", uns), (f"解けるが {TL // 6} 秒以上", slw)):
    a = [p for p in ps if p in pst]
    b = [p for p in a if p in big]
    print(f"{lab}: {len(ps)} 問、うち停滞 {len(a)}、うち停滞かつ残りギャップ > 10% {len(b)}: {', '.join(b)}")
