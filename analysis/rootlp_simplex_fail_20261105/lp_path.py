"""書き出した根の LP (MPS) を LP の経路 (`Model.solve(root_solver="simplex")`、LP の前処理あり、1 スレッド) で解く。
使い方: python lp_path.py 出力.jsonl 時間の上限 ラベル MPS...   環境変数はそのまま渡る (篩い分けの閾値など)。"""
import json, sys, time
from pathlib import Path
import highspy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
from run_mittelmann_benchmark import _build_our_model  # noqa: E402
out, tl, label = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
for f in sys.argv[4:]:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f)
    model = _build_our_model(h.getLp())
    t0 = time.perf_counter()
    r = model.solve("simplex", False, tl, None, None, 1, False)
    t = time.perf_counter() - t0
    rec = {"problem": Path(f).stem, "label": label, "t": round(t, 3), "status": r["status"], "obj": r.get("objective")}
    with out.open("a") as fo:
        fo.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
