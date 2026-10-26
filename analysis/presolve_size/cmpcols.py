"""HiGHS と ENOMOTO の前処理後の問題を列ごとに比べる (境界・型・費用・列の長さが違う列を出す)。
使い方: python3 cmpcols.py <元の mps> <HiGHS の前処理後> <ENOMOTO の前処理後> [出す数]"""
import sys, highspy, numpy as np
def load(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f); lp = h.getLp()
    names = list(lp.col_names_); st = lp.a_matrix_.start_; idx = lp.a_matrix_.index_; val = lp.a_matrix_.value_
    integ = list(lp.integrality_) if len(lp.integrality_) else [0] * lp.num_col_
    d = {}
    for j, nm in enumerate(names):
        d[nm] = dict(lo=lp.col_lower_[j], up=lp.col_upper_[j], c=lp.col_cost_[j], int=int(integ[j]) != 0,
                     coefs=sorted(abs(val[k]) for k in range(st[j], st[j + 1])))
    return names, d
on, od = load(sys.argv[1]); _, hd = load(sys.argv[2]); _, ed = load(sys.argv[3])
K = int(sys.argv[4]) if len(sys.argv) > 4 else 30
ed = {on[int(k[1:])]: v for k, v in ed.items()}  # ENOMOTO の列名 C<元の番号>
hfree = {k for k, v in hd.items() if v["lo"] < v["up"]}; efree = {k for k, v in ed.items() if v["lo"] < v["up"]}
print(f"HiGHS だけに残る列 {len(hfree - efree)}、ENOMOTO だけに残る列 {len(efree - hfree)}、両方 {len(hfree & efree)}")
for tag, s in (("ENOMOTO だけ", efree - hfree), ("HiGHS だけ", hfree - efree)):
    for k in sorted(s)[:K // 3]:
        v = ed.get(k) or hd.get(k); o = od[k]
        print(f"  {tag}: {k} 元[{o['lo']},{o['up']}] int={o['int']} c={o['c']:g} len={len(o['coefs'])} -> [{v['lo']},{v['up']}]")
nb = 0
for k in sorted(hfree & efree):
    h, e = hd[k], ed[k]
    if (h["lo"], h["up"], h["int"]) != (e["lo"], e["up"], e["int"]) or abs(h["c"] - e["c"]) > 1e-9 * max(1, abs(h["c"])) or len(h["coefs"]) != len(e["coefs"]):
        nb += 1
        if nb <= K:
            print(f"  違う: {k} H[{h['lo']},{h['up']}] int={h['int']} c={h['c']:g} len={len(h['coefs'])} max|a|={max(h['coefs'], default=0):g} | "
                  f"E[{e['lo']},{e['up']}] int={e['int']} c={e['c']:g} len={len(e['coefs'])} max|a|={max(e['coefs'], default=0):g}")
print(f"境界・型・費用・長さが違う共通の列 {nb}")
