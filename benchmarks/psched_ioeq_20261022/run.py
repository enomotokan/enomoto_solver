"""physiciansched6-2 の繰り返し計測 (手順 4) と、55 問での環境変数の効き目の確認 (手順 5)。

使い方:
  python run.py psched OUT.jsonl            # 2 設定 x 10 回 (交互)、4 スレッド・60 秒
  python run.py check OUT.jsonl             # 55 問 x (e2iohleq あり/なし)、4 スレッド・10 秒、ENOMOTO_MIP_LOG=1
  python run.py --worker MPS TL THREADS     # 子プロセス
どちらも OUT.jsonl に 1 行ずつ追記し、再実行すると済んだ分を飛ばす。
"""
import json, math, os, re, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CACHE = ROOT / ".miplib_cache"
E = {"ENOMOTO_MIP_PROBE_EQUIV2": "1", "ENOMOTO_MIP_EQUIV2_CUTS": "1", "ENOMOTO_MIP_EQUIV2_FJ": "1", "ENOMOTO_MIP_EQUIV2_TAB": "1"}
HL = {"ENOMOTO_MIP_ROOT_SUBMIP_HL": "1"}
IOEQ = {"ENOMOTO_MIP_INOUT_SEP": "1", "ENOMOTO_MIP_INOUT_EQ2_ONLY": "1"}
CONFIGS = {"e2hl": {**E, **HL}, "e2iohleq": {**E, **HL, **IOEQ}}
CHECK_CONFIGS = {"newdef": {}, "e2iohleq": CONFIGS["e2iohleq"]}
ALL_KEYS = set(k for c in CONFIGS.values() for k in c)


def worker(mps, tl, threads):
    from enomoto_solver import benchmark_highs as bh
    _, lp = bh._load_lp(Path(mps))
    m, _, _ = bh._build_our_model(lp)
    t = time.perf_counter()
    r = m.solve(None, False, tl, None, None, threads)
    t = time.perf_counter() - t
    x = r.get("x")
    chk = {}
    if x:
        n = lp.num_col_
        lo, up, integ = list(lp.col_lower_), list(lp.col_upper_), list(lp.integrality_)
        cost = list(lp.col_cost_)
        bviol = max([0.0] + [max(lo[j] - x[j], x[j] - up[j]) for j in range(n)])
        iviol = max([0.0] + [abs(x[j] - round(x[j])) for j in range(n) if len(integ) > j and int(integ[j]) != 0])
        am = lp.a_matrix_
        st, ix, va = list(am.start_), list(am.index_), list(am.value_)
        act = [0.0] * lp.num_row_
        if "kColwise" in str(am.format_):
            for j in range(n):
                for k in range(st[j], st[j + 1]):
                    act[ix[k]] += va[k] * x[j]
        else:
            for i in range(lp.num_row_):
                act[i] = sum(va[k] * x[ix[k]] for k in range(st[i], st[i + 1]))
        rl, ru = list(lp.row_lower_), list(lp.row_upper_)
        rviol = max([0.0] + [max(rl[i] - act[i], act[i] - ru[i]) for i in range(lp.num_row_)])
        obj = float(lp.offset_) + sum(cost[j] * x[j] for j in range(n))
        chk = {"bound_viol": float(bviol), "int_viol": float(iviol), "row_viol": float(rviol), "obj_recomputed": float(obj),
               "feasible": bool(bviol <= 1e-6 and iviol <= 1e-5 and rviol <= 1e-6),
               "obj_match": bool(abs(obj - r["objective"]) <= 1e-6 * max(1.0, abs(obj)))}
    print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r["best_bound"], "nodes": r["nodes"],
                      "time": t, "root_bound": r.get("root_bound"), "first_solution_time": r.get("first_solution_time"),
                      "first_solution_objective": r.get("first_solution_objective"), **chk}))


def run(mps, tl, threads, extra_env, log=False):
    env = {k: v for k, v in os.environ.items() if k not in ALL_KEYS and k != "ENOMOTO_MIP_LOG"}
    env.update(extra_env)
    env["ENOMOTO_MIP_THREADS"] = str(threads)
    if log:
        env["ENOMOTO_MIP_LOG"] = "1"
    cmd = [sys.executable, __file__, "--worker", str(mps), str(tl), str(threads)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=tl * 1.5 + 120, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "killed"}, ""
    r = None
    for line in reversed(p.stdout.splitlines()):
        if line.startswith("{"):
            r = json.loads(line)
            break
    if r is None:
        r = {"status": f"crash rc={p.returncode}", "stderr": p.stderr[-2000:]}
    w = list(dict.fromkeys(l.strip() for l in p.stderr.splitlines() if "MIP: WARNING" in l))
    if w:
        r["mip_warnings"] = sum("MIP: WARNING" in l for l in p.stderr.splitlines())
        r["mip_warning_text"] = w[:5]
    return r, p.stderr


def done_keys(out):
    s = set()
    if out.exists():
        for l in out.read_text().splitlines():
            if l.strip():
                d = json.loads(l)
                s.add((d.get("config"), d.get("rep"), d.get("problem")))
    return s


def psched(out):
    done = done_keys(out)
    for rep in range(10):
        for c, ev in CONFIGS.items():
            if (c, rep, "physiciansched6-2") in done:
                continue
            r, _ = run(CACHE / "physiciansched6-2.mps", 60.0, 4, ev)
            r.update(config=c, rep=rep, problem="physiciansched6-2", wall=time.strftime("%H:%M:%S"))
            with out.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(json.dumps(r), flush=True)


def parse_log(err):
    pres = [l.split("MIP: presolve: ", 1)[1] for l in err.splitlines() if "MIP: presolve: rows" in l]
    eq = [l.strip() for l in err.splitlines() if "equivalence pass with implications" in l]
    return {"presolve_lines": pres,
            "equiv2_used": any(l.endswith(", used") for l in eq),
            "equiv2_lines": eq[:3],
            "hl_rins": sum("RINS (HL)" in l for l in err.splitlines()),
            "hl_cuts": sum("root sub-MIP (HL)" in l for l in err.splitlines()),
            "inout_lines": sum("in-out" in l for l in err.splitlines()),
            "inout_text": [l.strip() for l in err.splitlines() if "in-out" in l][:3]}


def check(out):
    done = done_keys(out)
    for mps in sorted(CACHE.glob("*.mps")):
        for c in CHECK_CONFIGS:
            if (c, 0, mps.stem) in done:
                continue
            r, err = run(mps, 10.0, 4, CHECK_CONFIGS[c], log=True)
            r.update(parse_log(err))
            r.update(config=c, rep=0, problem=mps.stem)
            with out.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(mps.stem, c, r.get("status"), r.get("objective"), r["equiv2_used"], r["hl_rins"], r["inout_lines"], r["presolve_lines"][-1:] , flush=True)


if __name__ == "__main__":
    if sys.argv[1] == "--worker":
        worker(sys.argv[2], float(sys.argv[3]), int(sys.argv[4]))
    elif sys.argv[1] == "psched":
        psched(Path(sys.argv[2]))
    else:
        check(Path(sys.argv[2]))
