"""x_*.json の整数列を元の MPS で固定した MPS を書き出し、このクレートで解いて目的値と違反を見る (LP の許容差の漏れの確かめ)。
使い方: python fixmps.py x_combo_t1.json [環境変数 K=V ...]"""
import json, os, sys
import numpy as np, highspy
from pathlib import Path
R = Path(__file__).resolve().parents[2]
f = sys.argv[1]; os.environ.update(kv.split("=", 1) for kv in sys.argv[2:])
d = json.loads(Path(f).read_text())
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(str(R / ".miplib_cache/neos-5188808-nattai.mps"))
lp = h.getLp(); names = list(lp.col_names_)
for j, nm in enumerate(names):
    if int(lp.integrality_[j]) == 1:
        v = round(d["x"][nm]); h.changeColBounds(j, v, v)
out = Path(f).with_suffix(".fixed.mps"); h.writeModel(str(out))
from enomoto_solver import benchmark_highs as bh
m, _, _, nm2 = bh._read_our_model(out)
r = m.solve(None, False, 600, None, None, 1, False, None)
x = dict(zip(nm2, list(r["x"])))
Path(str(out) + ".x.json").write_text(json.dumps({"objective": r["objective"], "x": x}))
print("このクレート:", r["status"], repr(r["objective"]))
