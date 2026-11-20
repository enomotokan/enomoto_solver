"""同じ点での分離の比べ: 計装した HiGHS の各ラウンドの LP 解 x (hx_r<r>.txt) で、HiGHS が次のラウンドに出したカットと、当方の分離器が
同じ点で作った候補 (ENOMOTO_MIP_DEBUG_SEP_AT の x_e<k>.txt、x_r<k> = hx_r<k+1>) を、HiGHS がその時点までに出したカットを足した LP に
さらに足したときの LP の値で比べる。python sepat_cmp.py <mps> <ディレクトリ (hcuts.txt・x_e*.txt)> <点の番号,...>"""
import sys, math
sys.argv, args = sys.argv[:4] + ["-"], sys.argv
exec(open(__file__.replace("sepat_cmp.py", "cutlp.py")).read().split("for name, cuts in")[0].replace("mps, hc, ed = sys.argv[1:4]", "mps = sys.argv[1]; hc = sys.argv[2] + '/hcuts.txt'; ed = sys.argv[2]"))
H = []
for l in open(hc):
    t = l.split(); H.append((int(t[0]), int(t[1]), float(t[2]), [(int(a.split(":")[0]), float(a.split(":")[1])) for a in t[3:]]))
def le(k, src=None):
    out = []
    for l in open(f"{ed}/x_e{k}.txt"):
        t = l.split()
        if src is not None and int(t[0]) not in src: continue
        out.append((float(t[2]), [(int(a.split(":")[0]), float(a.split(":")[1])) for a in t[3:]]))
    return out
def viol(cuts, x):
    v = [(sum(a * x[j] for j, a in co) - r) / (math.sqrt(sum(a * a for _, a in co)) or 1) for r, co in cuts]
    v = [e for e in v if e > 1e-6]; return len(v), sum(v)
for k in map(int, args[3].split(",")):
    r = k + 1
    x = [float(l) for l in open(f"{ed}/x_r{k}.txt")]
    B = [(c[2], c[3]) for c in H if c[0] <= r]
    N = [(c[2], c[3]) for c in H if c[0] == r + 1]; Np = [(c[2], c[3]) for c in H if c[0] == r + 1 and c[1] == 3]
    E = le(k); Ep = le(k, {5, 6})
    v0, _ = lpval(B)
    res = {nm: lpval(B + S)[0] - v0 for nm, S in (("HiGHS 次", N), ("HiGHS 次 経路", Np), ("当方", E), ("当方 経路", Ep))}
    vs = {nm: viol(S, x) for nm, S in (("HiGHS 次 経路", Np), ("当方 経路", Ep))}
    print(f"点 {k} (HiGHS ラウンド {r} の後): 土台 {v0:.6e} ({len(B)} 本) | 伸び " + " ".join(f"{nm} {d:+.4g} ({len(S)})" for (nm, d), S in zip(res.items(), (N, Np, E, Ep)))
          + " | 破る本数・効き目の和 " + " ".join(f"{nm} {a}/{b:.3g}" for nm, (a, b) in vs.items()), flush=True)
