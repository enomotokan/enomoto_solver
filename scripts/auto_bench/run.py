"""ENOMOTO auto (二段解法と IPM+クロスオーバーの同時実行) vs HiGHS 疑似 auto
(双対単体法 parallel=on と IPX+クロスオーバーを別プロセスで同時に走らせ、先に結論を出した方を採る)。

時間は求解だけ (MPS 読み込み・モデル構築は含めない)。1 行ずつ JSONL に追記、再実行で再開。
スレッド数は論理 CPU 数 (ENOMOTO は RAYON_NUM_THREADS、HiGHS は各プロセスの threads)。
HiGHS 1.15.1 には単体法と内点法を同時に走らせる設定が無いので、2 プロセスで模している。

使い方 (リポジトリのルートで):
    python scripts/auto_bench/run.py --out benchmarks/auto_race.jsonl                  # netlib 93 + kennington 16 + mittelmann 小 15
    python scripts/auto_bench/run.py --sets mittelmann --out ...                       # 集合を絞る
    python scripts/auto_bench/report.py benchmarks/auto_race.jsonl                     # 集計
Mittelmann の圧縮データは scripts/run_mittelmann_benchmark.py の ensure_raw で取得する (.mittelmann_cache/raw)。
"""
from __future__ import annotations

import argparse, json, os, queue, shutil, statistics, subprocess, sys, tempfile, threading, time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
NCPU = os.cpu_count() or 1
MITT15 = ["qap15", "nug08-3rd", "fome13", "cont1", "cont11", "irish-electricity", "supportcase10", "Linf_520c",
          "rmine15", "physiciansched3-3", "neos", "ex10", "graph40-40", "pds-100", "ns1687037"]
CONCLUSIVE = {"optimal", "infeasible", "unbounded"}


def hstatus(h):
    import highspy
    S = highspy.HighsModelStatus
    return {S.kOptimal: "optimal", S.kInfeasible: "infeasible", S.kUnbounded: "unbounded",
            S.kTimeLimit: "timeout"}.get(h.getModelStatus(), str(h.getModelStatus()).split(".")[-1])


def worker_enomoto(mps: str) -> None:
    import highspy
    from enomoto_solver import _core
    from run_mittelmann_benchmark import _build_our_model
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
    model = _build_our_model(h.getLp()); del h
    t0 = time.perf_counter()
    out = model.solve(root_solver="auto", distinguish_infeasible_unbounded=True)
    t = time.perf_counter() - t0
    print("RESULT " + json.dumps({"time": t, "status": out["status"], "obj": out["objective"],
                                  "events": [[n, s] for n, s in _core.last_solve_events()]}), flush=True)


def worker_highs(mps: str, mode: str, limit: float) -> None:
    import highspy
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
    h.setOptionValue("threads", NCPU); h.setOptionValue("time_limit", limit)
    if mode == "simplex":
        h.setOptionValue("solver", "simplex"); h.setOptionValue("parallel", "on")
    else:
        h.setOptionValue("solver", "ipx"); h.setOptionValue("run_crossover", "on")
    print("READY", flush=True)
    sys.stdin.readline()
    t0 = time.perf_counter(); h.run(); t = time.perf_counter() - t0
    st = hstatus(h); info = h.getInfo()
    print("RESULT " + json.dumps({"time": t, "status": st, "obj": info.objective_function_value if st == "optimal" else None,
                                  "simplex_iters": info.simplex_iteration_count, "ipm_iters": info.ipm_iteration_count}), flush=True)


def _reader(name, p, q):
    for line in p.stdout:
        q.put((name, line.rstrip("\n")))
    q.put((name, None))


def run_enomoto(mps: Path, limit: float) -> dict:
    env = dict(os.environ, RAYON_NUM_THREADS=str(NCPU))
    try:
        p = subprocess.run([sys.executable, __file__, "--worker-enomoto", str(mps)], capture_output=True, text=True,
                           timeout=limit + 120, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "timeout", "time": limit}
    for line in p.stdout.splitlines():
        if line.startswith("RESULT "):
            r = json.loads(line[7:])
            if r["time"] > limit:
                r["status"] = "timeout"
            return r
    return {"status": "crash", "time": limit, "stderr": p.stderr[-1500:]}


def run_highs_race(mps: Path, limit: float) -> dict:
    q: queue.Queue = queue.Queue()
    procs = {}
    for mode in ("simplex", "ipx"):
        p = subprocess.Popen([sys.executable, __file__, "--worker-highs", str(mps), mode, str(limit)],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        procs[mode] = p
        threading.Thread(target=_reader, args=(mode, p, q), daemon=True).start()
    ready, results, dead = set(), {}, set()
    deadline = time.time() + limit + 600
    try:
        while len(ready | dead) < 2:
            name, line = q.get(timeout=max(1, deadline - time.time()))
            if line is None: dead.add(name)
            elif line == "READY": ready.add(name)
        for name in ready:
            procs[name].stdin.write("GO\n"); procs[name].stdin.flush()
        deadline = time.time() + limit + 60
        while len(results) + len(dead - set(results)) < 2:
            name, line = q.get(timeout=max(1, deadline - time.time()))
            if line is None:
                dead.add(name); continue
            if line.startswith("RESULT "):
                r = json.loads(line[7:]); results[name] = r
                if r["status"] in CONCLUSIVE:
                    return dict(r, winner=name, other={k: v for k, v in results.items() if k != name})
    except queue.Empty:
        pass
    finally:
        for p in procs.values():
            if p.poll() is None: p.kill()
    return {"status": "timeout" if not results else "fail", "time": limit, "detail": results}


def problem_list(sets):
    out = []
    for s in sets:
        if s == "netlib":
            out += [("netlib", n.strip()) for n in (REPO / ".netlib_cache/problems.txt").read_text().splitlines() if n.strip()]
        elif s == "kennington":
            out += [("kennington", n.strip()) for n in (REPO / ".kennington_cache/problems.txt").read_text().splitlines() if n.strip()]
        elif s == "mittelmann":
            out += [("mittelmann", n) for n in MITT15]
    return out


def materialize(s, name, workdir):
    if s != "mittelmann":
        return REPO / f".{s}_cache" / "mps" / f"{name}.mps"
    from run_mittelmann_benchmark import PROBLEMS, materialize_mps
    rel = next(p[1] for p in PROBLEMS if p[0] == name)
    return materialize_mps(REPO / ".mittelmann_cache", rel, REPO / ".mittelmann_cache/raw" / rel.replace("/", "__"), workdir)


def main():
    if sys.argv[1:2] == ["--worker-enomoto"]:
        return worker_enomoto(sys.argv[2])
    if sys.argv[1:2] == ["--worker-highs"]:
        return worker_highs(sys.argv[2], sys.argv[3], float(sys.argv[4]))
    ap = argparse.ArgumentParser()
    ap.add_argument("--sets", nargs="+", default=["netlib", "kennington", "mittelmann"])
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--single-run-above", type=float, default=30.0)
    ap.add_argument("--time-limit", type=float, default=600.0)
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()
    done = set()
    if a.out.exists():
        for l in a.out.read_text().splitlines():
            r = json.loads(l); done.add((r["set"], r["problem"]))
    for s, name in problem_list(a.sets):
        if (s, name) in done: continue
        (REPO / ".mittelmann_cache/tmp").mkdir(parents=True, exist_ok=True)
        wd = Path(tempfile.mkdtemp(dir=REPO / ".mittelmann_cache/tmp" if s == "mittelmann" else None))
        try:
            mps = materialize(s, name, wd)
            runs = {"enomoto": [], "highs": []}
            for rep in range(a.reps):
                order = ["enomoto", "highs"] if rep % 2 == 0 else ["highs", "enomoto"]
                for solver in order:
                    r = run_enomoto(mps, a.time_limit) if solver == "enomoto" else run_highs_race(mps, a.time_limit)
                    runs[solver].append(r)
                if rep == 0 and (max(runs["enomoto"][0]["time"], runs["highs"][0]["time"]) >= a.single_run_above
                                 or any(runs[k][0]["status"] not in CONCLUSIVE for k in runs)):
                    break
            rec = {"set": s, "problem": name, "runs": runs,
                   "median": {k: statistics.median(x["time"] for x in v) for k, v in runs.items()}}
        finally:
            shutil.rmtree(wd, ignore_errors=True)
        with open(a.out, "a") as f:
            f.write(json.dumps(rec) + "\n")
        e, h = runs["enomoto"][0], runs["highs"][0]
        print(f"{s:10s} {name:20s} enomoto {rec['median']['enomoto']:9.4f}s {e['status']:10s} "
              f"highs {rec['median']['highs']:9.4f}s {h['status']:10s} ({h.get('winner')})", flush=True)


if __name__ == "__main__":
    main()
