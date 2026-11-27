"""試作 `ENOMOTO_MIP_HO4_ACFIX=d` の誤答の確かめ用の fuzz: 行を足し合わせないと分からない「境界に張り付く列」を埋め込んだ小さな MIP を作り、
HiGHS と最適値・状態を突き合わせる (scripts/mip_fuzz_vs_highs.py の highs・ours を使う)。
使い方: ENOMOTO_MIP_HO4_ACFIX=dte python analysis/highsonly_20261123/fuzz_acfix.py 問題数 [開始seed]
(`t` で試作が働いた回数を数える 1 行が出る。`e` で根の LP の直後に必ず回す)

埋め込み方: 整数列の組 S と幅 1e5 の連続列の輪 T (長さ 2〜4) を選び、輪の各辺に
  y_i - y_{i+1} >= Σ_S a_ij x_j (σ = +1)、または y_i - y_{i+1} >= Σ_S a_ij (u_j - x_j) (σ = -1)
を足す。全部足すと 0 >= Σ (...) なので S の列は下限 (σ = +1) か上限 (σ = -1) に張り付く。2 行の形 (Σ_S a x + Σ_T y <= R と
Σ_T y >= R) はプロービングが根の LP の前に見つけてしまい試作の出番がなかったので、伝播では 1 回に 1 ずつしか締まらない輪にした。
ほかに普通のナップサック・被覆の行と、張り付かないが中心で境界に近い列 (幅の狭い組) を混ぜる。
目的は S の列を動かしたくなる向き (張り付きを誤って外すと良くなりすぎ、張り付かない列を誤って固定すると悪くなる)。"""
import random, sys, time
from pathlib import Path
src_path = Path(__file__).resolve().parents[2] / "scripts/mip_fuzz_vs_highs.py"
src = open(src_path).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, str(src_path), "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")


def gen(seed):
    g = random.Random(seed)
    nb = g.randint(6, 14)          # 0/1 列
    ni = g.randint(0, 4)           # 一般整数列 (0..U)
    nc = g.randint(2, 6)           # 連続列
    n = nb + ni + nc
    lo = [0.0] * n
    up = [1.0] * nb + [float(g.randint(2, 6)) for _ in range(ni)] + [round(g.uniform(1, 10), 2) for _ in range(nc)]
    it = [True] * (nb + ni) + [False] * nc
    c = [round(g.uniform(-10, 5), 2) for _ in range(n)]
    rows, rlo, rup = [], [], []
    ints = list(range(nb + ni)); conts = list(range(nb + ni, n))
    g.shuffle(ints); g.shuffle(conts)
    at0 = []  # 0 に張り付く列 (ナップサックに入れても実行可能のまま)
    used = set()
    for _ in range(g.randint(1, 3)):  # 張り付きの埋め込み (S・T は組ごとに別の列)
        k = g.randint(2, 4)  # 輪の長さ
        if len(ints) < 1 or len(conts) < k:
            break
        S = [ints.pop() for _ in range(min(len(ints), g.randint(1, 4)))]
        T = [conts.pop() for _ in range(k)]
        used.update(S); used.update(T)
        for t in T:  # 幅を大きくして、伝播 (1 回に 1 ずつ締まる) では矛盾にたどり着けないようにする
            up[t] = 1e5
        sg = g.choice([1.0, -1.0])
        # 輪: y_{T[i]} - y_{T[i+1]} >= σ Σ_S a_ij x_j (+ 上限側なら定数)。足すと 0 >= Σ_i Σ_S a_ij (x_j or u_j - x_j) → S は張り付く
        for i in range(k):
            a = {j: float(g.randint(1, 5)) for j in S}
            r = [(T[i], 1.0), (T[(i + 1) % k], -1.0)] + [(j, -sg * a[j]) for j in S]
            # sg > 0: y_i - y_{i+1} - Σ a x >= 0、sg < 0: y_i - y_{i+1} + Σ a x >= Σ a u (Σ a (u - x) <= y_i - y_{i+1})
            rows.append(r); rlo.append(0.0 if sg > 0 else sum(a[j] * up[j] for j in S)); rup.append(inf)
        if sg > 0:
            at0 += S
        for j in S:  # 目的は張り付きから離れたい向き
            c[j] = -abs(c[j]) - 1 if sg > 0 else abs(c[j]) + 1
    free = [j for j in range(n) if j not in used]
    pool = free + at0
    for _ in range(g.randint(1, 4)):  # ナップサック (自由な列と 0 に張り付く列)
        if len(pool) < 2:
            break
        xs = g.sample(pool, g.randint(2, min(6, len(pool))))
        rows.append([(j, float(g.randint(1, 9))) for j in xs]); rlo.append(-inf); rup.append(float(g.randint(5, 25)))
    for _ in range(g.randint(0, 2)):  # 被覆 (>=、自由な列だけ)
        if len(free) < 2:
            break
        xs = g.sample(free, g.randint(2, min(5, len(free))))
        rows.append([(j, float(g.randint(1, 4))) for j in xs]); rlo.append(1.0); rup.append(inf)
    fc = [j for j in free if not it[j]]
    if len(fc) >= 1 and g.random() < 0.5:  # 張り付かないが狭い連続列 (中心で境界に近い): Σ x <= 1e-4〜1e-2
        S = g.sample(fc, min(2, len(fc)))
        rows.append([(j, 1.0) for j in S]); rlo.append(-inf); rup.append(round(g.uniform(1e-4, 1e-2), 6))
    return lo, up, c, it, rows, rlo, rup


N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0; t0 = time.time(); stat = {}
for seed in range(start, start + N):
    lp = gen(seed); hs, ho = highs(lp); os_, oo, x = ours(lp)
    hmap = {"Optimal": "optimal", "Infeasible": "infeasible"}.get(hs, hs)
    stat[hmap] = stat.get(hmap, 0) + 1
    ok = (hmap == os_) and (hmap != "optimal" or abs(ho - oo) <= 1e-6 * (1 + abs(ho)))
    if not ok:
        bad += 1
        if bad <= 10: print("seed", seed, "highs", hs, ho, "ours", os_, oo, flush=True)
print("bad", bad, "of", N, "status", stat, "time %.1f" % (time.time() - t0))
