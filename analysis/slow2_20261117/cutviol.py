"""HiGHS の書き出したカット (hcutsrc の HIGHS_CUTSRC_DUMP、行: ラウンド 出どころ 右辺 列:係数 ...、sum a x <= 右辺) を、
当方の根の LP 解 (ENOMOTO_MIP_DEBUG_DUMP_ROOT_X の _x.txt、同じ列の番号) で評価する: python cutviol.py <カット> <x.txt>
出どころごとに本数、破られる本数 (効き目 = 違反量 / ||a|| > 1e-6)、効き目の和と上位、非零の数の平均を出す。"""
import sys, math, collections
x = [float(l) for l in open(sys.argv[2])]
N = ["implbd", "clique", "tableau", "path", "modk", "other"]
st = collections.defaultdict(lambda: [0, 0, 0.0, 0, []])
for l in open(sys.argv[1]):
    t = l.split(); r, s, rhs = int(t[0]), int(t[1]), float(t[2])
    co = [(int(a.split(":")[0]), float(a.split(":")[1])) for a in t[3:]]
    act = sum(a * x[j] for j, a in co); nrm = math.sqrt(sum(a * a for _, a in co)) or 1.0
    eff = (act - rhs) / nrm
    v = st[N[s]]; v[0] += 1; v[3] += len(co)
    if eff > 1e-6:
        v[1] += 1; v[2] += eff; v[4].append((eff, r, len(co)))
for k, v in st.items():
    top = sorted(v[4], reverse=True)[:5]
    print(f"{k}: {v[0]} 本 (平均非零 {v[3]/max(v[0],1):.1f})、破られる {v[1]} 本、効き目の和 {v[2]:.4g}、上位 (効き目, ラウンド, 非零) {[(round(a,4),b,c) for a,b,c in top]}")
