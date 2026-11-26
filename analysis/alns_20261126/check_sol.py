"""返した解と値の独立な確かめ: python check_sol.py スレッド数 秒 [設定 ...] (設定は cfg.py、既定は alns)。
問題 PROBS (pint.py と同じ、環境変数 ALNS_PROBS で替えられる) を当方で解き、返した解 x を、元の MPS を SCIP (pyscipopt) で読んだ問題に入れて
`checkSol` (境界・整数性・全部の制約、元の問題で) で確かめる。あわせて
- 目的値を SCIP で計算し直し、当方の値と比べる
- 既知の値 (scripts/miplib2017-v31.solu): =opt= なら返した目的値が最適値より良くないこと、返した下界 (最小化形) が最適値を越えないこと
を見る。結果は check_t<スレッド>.jsonl に足す (済んだ組は飛ばす)。子の過程で解くので、設定の環境変数は子だけに効く。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
from pint import PROBS, REF  # noqa: E402

CHILD = r"""
import json, sys
sys.path.insert(0, sys.argv[4])
import run_miplib_benchmark as rb
from enomoto_solver import benchmark_highs as bh
from pathlib import Path
m, _, _, names = rb._read_our_model_compat(bh, Path(sys.argv[1]))
r = m.solve(None, False, float(sys.argv[2]), None, None, int(sys.argv[3]))
print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r.get("best_bound"), "x": r["x"], "names": list(names)}))
"""


def check(p, res):
    """SCIP で元の問題を読み、返した解を確かめる"""
    import pyscipopt
    m = pyscipopt.Model()
    m.hideOutput()
    m.readProblem(str(R / ".miplib_cache" / f"{p}.mps"))
    out = {"feasible": None, "obj_scip": None, "obj_diff": None}
    if res["x"] is not None:
        byname = dict(zip(res["names"], res["x"]))
        sol = m.createSol()
        miss = 0
        for v in m.getVars():
            val = byname.get(v.name)
            if val is None:
                miss += 1
                continue
            m.setSolVal(sol, v, val)
        out["missing"] = miss
        out["feasible"] = bool(m.checkSol(sol, printreason=False, completely=True, checkbounds=True, checkintegrality=True, checklprows=True, original=True))
        out["obj_scip"] = m.getSolObjVal(sol, original=True)
        if res["objective"] is not None:
            out["obj_diff"] = abs(out["obj_scip"] - res["objective"]) / max(1.0, abs(res["objective"]))
    sense = 1.0 if m.getObjectiveSense() == "minimize" else -1.0
    ref = REF.get(p)
    out["ref"] = ref
    tol = lambda v: 1e-6 * max(1.0, abs(v))
    # 返した目的値が既知の値より良い: =opt= なら誤り (=best= なら新しい最良値の可能性)
    out["obj_better_than_ref"] = res["objective"] is not None and ref is not None and sense * (res["objective"] - ref) < -tol(ref)
    # 返した下界 (最小化形) が既知の値を越える: 最適値を切ったことになり誤り
    out["bound_above_ref"] = res.get("bound") is not None and ref is not None and sense * (res["bound"] - ref) > tol(ref)
    return out


if __name__ == "__main__":
    th, tl = int(sys.argv[1]), float(sys.argv[2])
    cfs = sys.argv[3:] or ["alns"]
    opt_names = set()
    for l in open(R / "scripts/miplib2017-v31.solu"):
        t = l.split()
        if len(t) >= 2 and t[0] == "=opt=":
            opt_names.add(t[1])
    outp = D / f"check_t{th}.jsonl"
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(outp)} if outp.exists() else set()
    for p in PROBS:
        for c in cfs:
            if (p, c) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFGS[c]); env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1"})
            r = subprocess.run([sys.executable, "-c", CHILD, str(R / ".miplib_cache" / f"{p}.mps"), str(tl), str(th), str(R / "scripts")],
                               cwd=R, env=env, capture_output=True, text=True, timeout=tl + 600)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            ck = check(p, res)
            is_opt = p in opt_names
            bad = (ck["feasible"] is False) or (ck["obj_diff"] is not None and ck["obj_diff"] > 1e-6) or (is_opt and ck["obj_better_than_ref"]) or ck["bound_above_ref"]
            row = {"problem": p, "cfg": c, "threads": th, "tl": tl, "status": res["status"], "objective": res["objective"], "bound": res["bound"], "ref_is_opt": is_opt, **ck, "bad": bad}
            with open(outp, "a") as f:
                f.write(json.dumps(row) + "\n")
            print(th, c, p, res["status"], res["objective"], "feasible", ck["feasible"], "objdiff", ck["obj_diff"], "BAD" if bad else "ok", flush=True)
