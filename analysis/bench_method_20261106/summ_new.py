"""新しい集計を 1 つの bench にかける。
使い方: python3 summ_new.py '<jsonl の glob (analysis からの相対)>' A B [B2 ...]
例:     python3 summ_new.py 'halve_submip_20261030/bench29_s*.jsonl' def s1 s3 s6 all29
回が 100 以上の鍵は Mittelmann、それ未満は 58 問として別に出す (halve_submip の分担の決まり)。揺れの表は noise58.json・noiseMitt.json。"""
import sys
from pathlib import Path
AN = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(AN))
import benchlib as bl  # noqa: E402

pat, A, Bs = sys.argv[1], sys.argv[2], sys.argv[3:]
R = bl.table(bl.load(sorted(AN.glob(pat))))
n58, nM = bl.load_noise("noise58.json"), bl.load_noise("noiseMitt.json")
for lab, sel, nz in (("58 問", lambda k: k[1] < 100, n58), ("Mittelmann", lambda k: k[1] >= 100, nM)):
    keys = [k for k in R if isinstance(k[1], int) and sel(k)] if any(isinstance(k[1], int) for k in R) else list(R)
    if not keys:
        continue
    if len(Bs) > 1:
        m, top = bl.def_luck(R, A, Bs, keys)
        if top:
            sa = bl.sgm([bl.t(R[k][A]) for k in keys if A in R[k]])
            print(f"*** {lab}: 既定の回の運 {m:+.4f} (SGM 相当 {bl.sgm_pct(sa, m):+.1%}、正 = 既定の回が遅い)  "
                  + ", ".join(f"{k[0]} r{k[1]} {x:+.2f}" for x, k in top[:4]))
    for B in Bs:
        bl.report(R, A, B, keys, noise=nz, label=f"{lab} {B}")
        print()
