"""ENOMOTO_DEBUG_LU_CAL=1 のログ (LU_CAL 行と PROF_PHASES_EXT) から、分解と求解の 1 手間あたりの秒を比べ、分解の秒を m・nnz(L+U)・消去の積和で当てはめる。
使い方: python lucal.py ログ ... (README 9.6 節)"""
import re, sys
import numpy as np
rows = []
for f in sys.argv[1:]:
    t = open(f).read()
    L = [dict(re.findall(r"(\w+)=([\d.e+-]+)", l)) for l in t.splitlines() if l.startswith("LU_CAL")]
    L = [{k: float(v) for k, v in d.items()} for d in L]
    bsec = sum(d["secs"] for d in L); btk = sum(d["build_tick"] for d in L); stk = sum(d["prev_solve_tick"] for d in L)
    # 主ループの LU の求解の秒 (FTRAN・BTRAN・FT 更新・DSE 更新)
    ssec = sum(float(x) for x in re.findall(r"^\s+(?:ftran|btran\(rho_p\)|ft_update|dse_update)\s+([\d.]+)ms", t, re.M)) / 1e3
    print(f"{f.split('/')[-1]:28s} 分解 {len(L):4d} 回 {bsec:7.2f} 秒 build_tick {btk:.3e} → {bsec / max(btk, 1) * 1e9:7.2f} ns/tick | 求解 {ssec:7.2f} 秒 tick {stk:.3e} → {ssec / max(stk, 1) * 1e9:6.2f} ns/tick")
    rows += [(d["m"], d["nnz"], d["flops"], d["secs"]) for d in L if d["secs"] > 0]
A = np.array([[r[0], r[1], r[2]] for r in rows]); y = np.array([r[3] for r in rows])
w = 1 / np.maximum(y, 1e-4)  # 相対誤差で当てる
c, *_ = np.linalg.lstsq(A * w[:, None], y * w, rcond=None)
print("当てはめ (秒 = a·m + b·nnz + c·flops、相対誤差の最小二乗): a = %.3e  b = %.3e  c = %.3e" % tuple(c))
pred = A @ c
print("比 (予測/実測) の中央値 %.2f、10〜90%% %.2f〜%.2f" % (np.median(pred / y), np.percentile(pred / y, 10), np.percentile(pred / y, 90)))
