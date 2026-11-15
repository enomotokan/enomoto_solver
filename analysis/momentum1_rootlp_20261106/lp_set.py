"""書き出した LP (MPS) を HiGHS (双対単体法・IPX、前処理 on/off、尺度 on/off) と ENOMOTO の LP の経路 (前処理あり) で解く。
使い方: python lp_set.py 出力.jsonl 時間の上限 ラベル MPS 解き方...
解き方: hs_<pre>_<scale> (双対単体法、pre=on/off、scale=on/off)、hipx_<pre> (IPX + クロスオーバー)、en_simplex、en_ipm (ipm_crossover)、en_auto。
環境変数はそのまま ENOMOTO に渡る。"""
import json, sys, time
from pathlib import Path
import highspy
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
from run_mittelmann_benchmark import _build_our_model  # noqa: E402
out, tl, label, f = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3], sys.argv[4]
for how in sys.argv[5:]:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f)
    rec = {"lp": Path(f).stem, "label": label, "how": how}
    if how.startswith("hs_") or how.startswith("hipx_"):
        p = how.split("_")
        h.setOptionValue("time_limit", tl); h.setOptionValue("threads", 1)
        h.setOptionValue("presolve", p[1])
        if how.startswith("hs_"):
            h.setOptionValue("solver", "simplex"); h.setOptionValue("simplex_strategy", 1)
            if p[2] == "off":
                h.setOptionValue("simplex_scale_strategy", 0)
        else:
            h.setOptionValue("solver", "ipm"); h.setOptionValue("run_crossover", "on")
        t0 = time.perf_counter(); h.run(); t = time.perf_counter() - t0
        info = h.getInfo()
        rec.update(t=round(t, 3), status=h.modelStatusToString(h.getModelStatus()), obj=info.objective_function_value,
                   iters=info.simplex_iteration_count, ipm_iters=info.ipm_iteration_count)
    else:
        model = _build_our_model(h.getLp())
        rs = {"en_simplex": ("simplex", 1), "en_ipm": ("ipm_crossover", 4), "en_auto": (None, 4)}[how]
        t0 = time.perf_counter()
        r = model.solve(rs[0], False, tl, None, None, rs[1], False)
        t = time.perf_counter() - t0
        rec.update(t=round(t, 3), status=r["status"], obj=r.get("objective"))
    with out.open("a") as fo:
        fo.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
