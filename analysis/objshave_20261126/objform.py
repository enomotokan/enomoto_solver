"""240 問 (analysis/gap240_20261126/gap.json) の目的の形を MPS から数える: 目的の非零の数、そのうち 0-1 列・一般整数・連続の数、
係数がみな整数か (と係数の gcd)、目的が整数値を取るか (目的の列がみな整数で係数が整数)、係数の種類の数。python objform.py → objform.jsonl"""
import json, math, sys
from fractions import Fraction
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
gap = json.loads((WT / "analysis/gap240_20261126/gap.json").read_text())
OUT = D / "objform.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for g in gap:
    p = g["problem"]
    if p in done: continue
    row = {"problem": p}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
        h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
        lp = h.getLp(); c = list(lp.col_cost_); lo = list(lp.col_lower_); up = list(lp.col_upper_)
        integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * len(c)
        nb = ni = nc = 0; coefs = []
        for j, cj in enumerate(c):
            if cj == 0: continue
            isint = integ[j] != highspy.HighsVarType.kContinuous
            if isint and lo[j] >= 0 and up[j] <= 1: nb += 1
            elif isint: ni += 1
            else: nc += 1
            coefs.append(cj)
        intc = all(abs(x - round(x)) < 1e-9 for x in coefs)
        gcd = 0
        if intc:
            for x in coefs: gcd = math.gcd(gcd, int(round(abs(x))))
        row.update({"n": len(c), "obj_nnz": len(coefs), "obj_bin": nb, "obj_int": ni, "obj_cont": nc, "int_coef": intc, "gcd": gcd,
                    "obj_integral": intc and nc == 0, "distinct": len({round(x, 9) for x in coefs})})
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
