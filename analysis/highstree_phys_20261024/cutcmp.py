"""HiGHS と ENOMOTO の 1 ラウンド目のカットを、同じ LP (ENOMOTO の前処理後の問題 eno_pre の LP 緩和) に足して比べる。
python -I cutcmp.py <mps> <HiGHS の h_cuts_r1.txt> <HiGHS の h_x_r1.txt> <ENOMOTO の cands_r0.txt> <ENOMOTO の chosen_r0.txt>
カットはどちらも sum a x <= rhs。HiGHS: `分離器 右辺 整数 列:係数 ...`、ENOMOTO: `分離器の番号 効き目 右辺 列:係数 ...`。"""
import sys
from collections import defaultdict
import highspy, numpy as np
mps, hc, hx, ec, ech = (sys.argv[1:6] if __name__ == "__main__" else LIBARGS)
ENAMES = ["extra", "rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "pool"]
def read_h(f):
    out = []
    for line in open(f):
        t = line.split()
        out.append((t[0], float(t[1]), [(int(a), float(b)) for a, b in (x.split(":") for x in t[3:])]))
    return out
def read_e(f):
    out = []
    for line in open(f):
        t = line.split()
        s = int(t[0]); out.append((ENAMES[s % 16] + ("@face" if s >= 16 else ""), float(t[2]), [(int(a), float(b)) for a, b in (x.split(":") for x in t[3:])]))
    return out
H = read_h(hc) if hc else []; E = read_e(ec); EC = read_e(ech) if ech else []
x = np.array([float(v) for v in open(hx)][1:])
base = highspy.Highs(); base.setOptionValue("output_flag", False); base.readModel(mps)
lp0 = base.getLp()
lp0.integrality_ = [highspy.HighsVarType.kContinuous] * lp0.num_col_
def lp_with(cuts):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1); h.passModel(lp0)
    if cuts:
        starts, idx, val, ub = [], [], [], []
        for _, r, cs in cuts:
            starts.append(len(idx)); idx += [j for j, _ in cs]; val += [a for _, a in cs]; ub.append(r)
        h.addRows(len(cuts), np.full(len(cuts), -highspy.kHighsInf), np.array(ub), len(idx), np.array(starts, dtype=np.int32), np.array(idx, dtype=np.int32), np.array(val))
    h.run()
    return h.getInfo().objective_function_value, h.getInfo().simplex_iteration_count
def stats(cuts):
    L = [len(c) for _, _, c in cuts]
    eff = []
    for _, r, cs in cuts:
        act = sum(a * x[j] for j, a in cs); nrm = np.sqrt(sum(a * a for _, a in cs))
        eff.append((act - r) / nrm if nrm > 0 else 0)
    eff = np.array(eff)
    return f"本数 {len(cuts)} 非零の平均 {np.mean(L):.1f} (中央 {np.median(L):.0f}, 最大 {max(L)}) HiGHS の x での違反 > 1e-6 の本数 {int((eff > 1e-6).sum())} 効き目の最大 {eff.max():.3g} 中央 {np.median(eff):.3g}"
if __name__ == "__main__":
    print("LP 緩和", lp_with([]))
    for name, cuts in [("HiGHS 全部", H), ("ENOMOTO 候補 全部", E), ("ENOMOTO 採用 (chosen)", EC)]:
        print(f"{name}: {stats(cuts)} -> LP {lp_with(cuts)}")
    for name, cuts in [("HiGHS", H), ("ENOMOTO 候補", E), ("ENOMOTO 採用", EC)]:
        g = defaultdict(list)
        for c in cuts:
            g[c[0]].append(c)
        for k, cs in sorted(g.items(), key=lambda kv: -len(kv[1])):
            print(f"  {name} / {k}: {stats(cs)} -> LP {lp_with(cs)[0]:.6g}")
