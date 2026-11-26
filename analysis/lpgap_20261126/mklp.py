"""比べに使う LP を MPS で書き出す: python mklp.py
- dano3_3_fix: dano3_3 の整数列 (69 本) を当方の 1 スレッドの最適解の値で固定した LP
- dano3_3_rel・physiciansched6-2_rel など: 整数を外した LP 緩和
出力は .lpgap_cache/ (gitignore されない作業用の場所なので、コミットしない)。"""
import json, sys
from pathlib import Path
import highspy
R = Path(__file__).resolve().parents[2]
C = R / ".miplib_cache"; O = R / ".lpgap_cache"; O.mkdir(exist_ok=True)


def relax(name):
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(str(C / f"{name}.mps"))
    lp = h.getLp(); n = lp.num_col_
    for j in range(n):
        h.changeColIntegrality(j, highspy.HighsVarType.kContinuous)
    return h


def write_rel(name):
    h = relax(name); h.writeModel(str(O / f"{name}_rel.mps")); print("書いた", name + "_rel")


def write_fix(name, xfile):
    d = json.loads(Path(xfile).read_text())
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    h.readModel(str(C / f"{name}.mps"))
    lp = h.getLp(); names = list(lp.col_names_); integ = list(lp.integrality_); k = 0
    for j, nm in enumerate(names):
        if int(integ[j]) == 1:
            v = round(d[nm]); h.changeColBounds(j, v, v); h.changeColIntegrality(j, highspy.HighsVarType.kContinuous); k += 1
    h.writeModel(str(O / f"{name}_fix.mps")); print("書いた", name + "_fix", "固定", k)


if __name__ == "__main__":
    for a in sys.argv[1:]:
        if a.startswith("fix:"):
            _, nm, xf = a.split(":"); write_fix(nm, xf)
        else:
            write_rel(a)
