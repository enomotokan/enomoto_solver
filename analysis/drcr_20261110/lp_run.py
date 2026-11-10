"""DRCR の LP の計測 (analysis/drcr_20261110.md)。

使い方:
  python analysis/drcr_20261110/lp_run.py worker <mps>            # 1 回解いて JSON を 1 行 (環境変数 ENOMOTO_LP_FOLD で切り替え)
  python analysis/drcr_20261110/lp_run.py run <out.jsonl> <cfg> <timeout> <mps...>
    cfg: def (なし) / fold (ENOMOTO_LP_FOLD=1) / rfold (=r) / count (=r と 1 の数だけ、ENOMOTO_T_FOLD_MAX_RATIO=1)
整数列は連続として読む (MIPLIB は LP 緩和)。HiGHS の目的値も記録する (ref)。
"""
import json, math, os, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))


def worker(mps):
    import highspy
    from run_mittelmann_benchmark import _build_our_model
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(mps)
    lp = h.getLp()
    lp.integrality_ = []
    m = _build_our_model(lp)
    t0 = time.perf_counter()
    out = m.solve(root_solver=None, threads=int(os.environ.get("LPTHREADS", "4")))
    t = time.perf_counter() - t0
    print(json.dumps({"time": t, "status": out["status"], "obj": out.get("objective")}), flush=True)


def highs_ref(mps, timeout):
    import highspy
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", float(timeout))
    h.readModel(mps)
    lp = h.getLp()
    lp.integrality_ = []
    h.passModel(lp)
    t0 = time.perf_counter()
    h.run()
    return h.getInfo().objective_function_value if h.getModelStatus() == highspy.HighsModelStatus.kOptimal else None, time.perf_counter() - t0


CFG = {"def": {}, "fold": {"ENOMOTO_LP_FOLD": "1"}, "rfold": {"ENOMOTO_LP_FOLD": "r"}, "rfoldv": {"ENOMOTO_LP_FOLD": "rv"}, "foldv": {"ENOMOTO_LP_FOLD": "1v"}}


def run(out, cfg, timeout, mpss):
    done = set()
    if os.path.exists(out):
        for l in open(out):
            d = json.loads(l)
            done.add((d["name"], d["cfg"]))
    for mps in mpss:
        name = Path(mps).name.split(".")[0]
        if (name, cfg) in done:
            continue
        env = dict(os.environ, **CFG[cfg])
        t0 = time.time()
        try:
            p = subprocess.run([sys.executable, __file__, "worker", mps], env=env, capture_output=True, text=True, timeout=timeout + 30)
            res = {}
            for l in p.stdout.splitlines():
                if l.startswith("{"):
                    res = json.loads(l)
            if not res:
                res = {"status": "crash", "rc": p.returncode}
            err = [l for l in p.stderr.splitlines() if l.startswith("LP_FOLD")]
        except subprocess.TimeoutExpired:
            res, err = {"status": "timeout"}, []
        res.update(name=name, cfg=cfg, wall=time.time() - t0, fold=err)
        with open(out, "a") as f:
            f.write(json.dumps(res) + "\n")
        print(name, cfg, res.get("status"), round(res.get("time", -1), 3), err[:1], flush=True)


if __name__ == "__main__":
    if sys.argv[1] == "worker":
        worker(sys.argv[2])
    elif sys.argv[1] == "run":
        run(sys.argv[2], sys.argv[3], float(sys.argv[4]), sys.argv[5:])
