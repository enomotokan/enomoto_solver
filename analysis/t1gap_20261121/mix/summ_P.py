"""P の 1 スレッド比較の集計: 設定ごとに解けた数・時間の SGM (ずれ 1 秒、解けない回は上限)・目的値の食い違い・
両方解けなかった回の暫定解の良し悪し。python summ_P.py [基準 def_t1] [組 P (res_P_s*.jsonl)、P2 なら res_P2_s*.jsonl]"""
import json, sys, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
SETN = sys.argv[2] if len(sys.argv) > 2 else "P"
for f in sorted(D.glob(f"res_{SETN}_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["seed"], r["cfg"])] = r
base = sys.argv[1] if len(sys.argv) > 1 else "def_t1"
cfgs = sorted({k[2] for k in R}, key=lambda c: (c != base, c))
keys = sorted({k[:2] for k in R if all((k[0], k[1], c) in R for c in cfgs)})
def ok(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def t(r): return min(r["result"].get("time", 60), 60) if ok(r) else 60
def sense():  # 最小化として扱う (MIPLIB の P は最小化のみ)
    return 1
print("全設定がそろった組:", len(keys))
for c in cfgs:
    rs = [R[(p, s, c)] for p, s in keys]
    sgm = math.exp(sum(math.log(t(r) + 1) for r in rs) / len(rs)) - 1
    print(f"{c:10s} 解けた {sum(ok(r) for r in rs):3d}  SGM {sgm:6.2f}  ノード計 {sum(r['result'].get('nodes',0) or 0 for r in rs)}")
for c in cfgs[1:]:
    w = l_ = 0; ib = iw = 0; mis = []; d = []
    for p, s in keys:
        a, b = R[(p, s, base)], R[(p, s, c)]
        if ok(a) and ok(b):
            oa, ob = a["result"].get("objective"), b["result"].get("objective")
            if oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)): mis.append((p, s, oa, ob))
        ta, tb = t(a), t(b)
        if ok(a) or ok(b): d.append((p, s, ta, tb))
        if tb < ta / 1.1 - 0.5: w += 1
        elif tb > ta * 1.1 + 0.5: l_ += 1
        if not ok(a) and not ok(b):
            oa, ob = a["result"].get("objective"), b["result"].get("objective")
            if oa is None and ob is not None: ib += 1
            elif ob is None and oa is not None: iw += 1
            elif oa is not None and ob is not None:
                if ob < oa - 1e-6 * max(1, abs(oa)): ib += 1
                elif ob > oa + 1e-6 * max(1, abs(oa)): iw += 1
    print(f"\n{c} 対 {base}: 速い {w} 遅い {l_} (1.1 倍かつ 0.5 秒超)、両方未解決の暫定解 良 {ib} 悪 {iw}、目的値の食い違い {len(mis)}")
    for m in mis: print("  食い違い", m)
    d.sort(key=lambda x: (x[3] + 1) / (x[2] + 1))
    print("  速くなった上位:", [(p, s, round(a, 1), round(b, 1)) for p, s, a, b in d[:6]])
    print("  遅くなった上位:", [(p, s, round(a, 1), round(b, 1)) for p, s, a, b in d[::-1][:6]])
