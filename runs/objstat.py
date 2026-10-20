# 40 問の目的関数の形: 整数列の数、費用のある整数列の数、費用のある連続列の数 (試作の条件に当たるかの目安)
import sys, gzip, urllib.request, highspy, numpy as np
from pathlib import Path
sys.path.insert(0, "scripts")
from run_miplib_benchmark import PROBLEMS, BASE_URL
cache = Path(".miplib_cache")
for name in PROBLEMS:
    mps = cache / f"{name}.mps"
    if not mps.exists():
        gz = cache / f"{name}.mps.gz"
        if not gz.exists():
            with urllib.request.urlopen(BASE_URL + f"{name}.mps.gz", timeout=120) as r, open(gz, "wb") as f: f.write(r.read())
        mps.write_bytes(gzip.decompress(gz.read_bytes()))
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp(); c = np.array(lp.col_cost_); integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(lp.num_col_)
    isint = integ != 0
    ci = int(((c != 0) & isint).sum()); cc = int(((c != 0) & ~isint).sum())
    print(f"{name:24s} nint {int(isint.sum()):6d} int_cost {ci:6d} ({ci/max(1,isint.sum()):.3f}) cont_cost {cc:5d}", flush=True)
