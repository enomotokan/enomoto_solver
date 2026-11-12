"""detect.jsonl の集計 → detect_summ.txt。is_int: 既定 (整数列だけ)、row: 行の規則 (OBJ_STEP_IMPLINT)、ho2: HO2 の規則、
hf: OBJINT=1 (HO2 + 整数値に固定された列)、highs: HiGHS の 1/scale。既定と OBJINT=1 で違う問題と、HiGHS と違う問題を出す。"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
def g(r, k):
    m = re.search(rf"\b{k} (None|Some\(([-0-9.e]+)\))", " ".join(r.get("ours", [])))
    return None if not m or m.group(1) == "None" else float(m.group(2))
out = []
rs = [json.loads(l) for l in open(D / "detect.jsonl")]
eq = lambda a, b: (a is None and b is None) or (a is not None and b is not None and abs(a - b) <= 1e-6 * max(abs(a), abs(b)))
out.append(f"{'問題':28s} {'is_int':>7s} {'row':>7s} {'ho2':>7s} {'hf':>7s} {'highs':>9s}  印")
nchg = nmis = 0
for r in rs:
    s0, row, ho2, hf = g(r, "step0"), g(r, "step_rowrule"), g(r, "step_ho2"), g(r, "step_hf")
    hs = r.get("highs_scale"); h = 1 / hs if hs else None
    mark = []
    if not eq(s0, hf): mark.append("OBJINTで変わる"); nchg += 1
    if not eq(hf, h): mark.append("HiGHSと違う"); nmis += 1
    f = lambda v: "-" if v is None else f"{v:g}"
    out.append(f"{r['problem']:28s} {f(s0):>7s} {f(row):>7s} {f(ho2):>7s} {f(hf):>7s} {f(h):>9s}  {' '.join(mark)}")
out.append(f"問題 {len(rs)}、OBJINT=1 で刻みが変わる {nchg}、OBJINT=1 と HiGHS が違う {nmis}")
(D / "detect_summ.txt").write_text("\n".join(out) + "\n")
print("\n".join(l for l in out if "印" in l or "変わる" in l or "違う" in l))
