"""b58k のベイズ最適化の次の提案: python propose.py R K。
これまでの結果 (bo_s*.jsonl) から設定ごとの目的値を出し、ガウス過程 (RBF、長さのスケールは格子で周辺尤度最大) + 期待改善量 (EI) で、
Kriging believer の batch として K 点を提案して proposals_rR.json に書く。R = 0 は初期の点 (今の =4 の既定 + ラテン超方格の K − 1 点)。
目的値 (小さいほど良い) = 回ごとの log(秒 + 10) の平均。秒は 600 で頭打ち、解けない回は 2 × 600 秒 (罰)。乱数の種は R で固定。"""
import glob, json, math, sys
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
from space import BASELINE, DIM, to_env  # noqa: E402

D = Path(__file__).resolve().parent
TL = 600.0


def objective_table():
    """{設定名: (x, 目的値, 回数)}"""
    props = {}
    for f in sorted(D.glob("proposals_r*.json")):
        props.update(json.loads(f.read_text())["cfgs"])
    vals = {}
    for f in sorted(D.glob("bo*_s*.jsonl")):
        for l in open(f):
            if not l.strip():
                continue
            r = json.loads(l)
            st = (r["result"].get("status") or "").lower()
            t = min(TL, r["result"].get("time", TL))
            if st != "optimal":
                t = 2 * TL
            vals.setdefault(r["cfg"], []).append(math.log(t + 10))
    out = {}
    for c, v in vals.items():
        if c in props:
            out[c] = (props[c]["x"], sum(v) / len(v), len(v))
    return out


def gp_fit(X, y):
    ym, ys = y.mean(), y.std() + 1e-9
    z = (y - ym) / ys
    best = None
    for ell in (0.1, 0.2, 0.3, 0.5, 0.8):
        for noise in (1e-3, 1e-2, 1e-1):
            K = np.exp(-0.5 * ((X[:, None, :] - X[None, :, :]) ** 2).sum(-1) / ell ** 2) + noise * np.eye(len(X))
            try:
                L = np.linalg.cholesky(K)
            except np.linalg.LinAlgError:
                continue
            a = np.linalg.solve(L.T, np.linalg.solve(L, z))
            ll = -0.5 * z @ a - np.log(np.diag(L)).sum()
            if best is None or ll > best[0]:
                best = (ll, ell, noise, L, a)
    _, ell, noise, L, a = best
    def pred(Xs):
        k = np.exp(-0.5 * ((Xs[:, None, :] - X[None, :, :]) ** 2).sum(-1) / ell ** 2)
        mu = k @ a
        v = np.linalg.solve(L, k.T)
        var = np.maximum(1.0 - (v ** 2).sum(0), 1e-12)
        return mu * ys + ym, np.sqrt(var) * ys
    return pred, ell, noise


def ei(mu, sd, best):
    z = (best - mu) / sd
    cdf = 0.5 * (1 + np.vectorize(math.erf)(z / math.sqrt(2)))
    pdf = np.exp(-0.5 * z ** 2) / math.sqrt(2 * math.pi)
    return (best - mu) * cdf + sd * pdf


def main():
    R, K = int(sys.argv[1]), int(sys.argv[2])
    rng = np.random.default_rng(1000 + R)
    if R == 0:
        pts = [list(BASELINE)]
        n = K - 1
        lhs = np.stack([(rng.permutation(n) + rng.random(n)) / n for _ in range(DIM)], 1)
        pts += lhs.tolist()
        info = "初期の点"
    else:
        tab = objective_table()
        X = np.array([v[0] for v in tab.values()], float)
        y = np.array([v[1] for v in tab.values()], float)
        cand = rng.random((4096, DIM))
        pts = []
        Xb, yb = X.copy(), y.copy()
        for _ in range(K):
            pred, ell, noise = gp_fit(Xb, yb)
            mu, sd = pred(cand)
            e = ei(mu, sd, yb.min())
            i = int(np.argmax(e))
            pts.append(cand[i].tolist())
            # Kriging believer: 予測の平均を観測とみなして足す
            Xb = np.vstack([Xb, cand[i]])
            yb = np.append(yb, mu[i])
            cand = np.delete(cand, i, 0)
        info = f"GP-EI (長さのスケール {ell}、ノイズ {noise}、これまで {len(X)} 設定、最良 {y.min():.4f})"
    cfgs = {f"r{R}_{i}": {"x": [round(v, 4) for v in p], "env": to_env(p)} for i, p in enumerate(pts)}
    (D / f"proposals_r{R}.json").write_text(json.dumps({"round": R, "info": info, "cfgs": cfgs}, indent=1, ensure_ascii=False))
    print(info)
    for c, v in cfgs.items():
        print(c, v["x"])


if __name__ == "__main__":
    main()
