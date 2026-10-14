"""Mittelmann LPopt の全問で ENOMOTO auto / HiGHS 疑似 auto / CLP を比べる。

  ENOMOTO: solve(root_solver="auto", threads=T, time_limit=L)
  HiGHS:   双対単体法 (parallel=on) と IPX+クロスオーバーを別プロセスで同時に走らせ、先に結論を出した方を採る
           (HiGHS 1.15.1 には単体法と内点法を同時に走らせる設定が無いので、各プロセス threads=T)
  CLP:     WSL の論文用ドライバ (scripts/paper_bench/drivers/clp_driver.cpp、既定の ClpSolve、逐次)

時間は求解だけ (MPS 読み込み・モデル構築は含めない)。問題は nnz の小さい順。

途中で止まっても結果が残るように:
  - 1 回の求解ごとに --out (JSONL) へ追記し、問題が終わるたびに要約 (--out の .md) を書き直す。
  - 同じコマンドで再開できる (終わった問題は飛ばす。途中の問題はやり直す)。
  - 最初に圧縮データをまとめて取得しておく (ディスクの空きが --min-free-gb を割らない範囲で)。
    取得できなかった問題は、その問題の番が来たときにもう一度取りにいく。
  - 計測中は Windows のスリープを抑止する。
  - ディスクの空きが足りず展開できない問題は飛ばし (skip、reason "disk")、再開時にもう一度試す。

使い方 (リポジトリのルートで):
    python scripts/auto_bench/mitt_full.py --out benchmarks/mitt_full_16t_600s.jsonl
"""
from __future__ import annotations

import argparse, bz2, json, math, os, platform, queue, shutil, statistics, subprocess, sys, tempfile, threading, time
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
from run_mittelmann_benchmark import PROBLEMS, BASE_URL, ensure_raw, materialize_mps  # noqa: E402

CONCLUSIVE = {"optimal", "infeasible", "unbounded"}
CACHE = REPO / ".mittelmann_cache"
CLP_BIN = "/home/enomotokan/paper_solvers/bin/clp_driver"
SHIFT = 10.0


# ---------------------------------------------------------------- workers (子プロセス)

def worker_enomoto(mps: str, threads: int, limit: float) -> None:
    import highspy
    from enomoto_solver import _core
    from run_mittelmann_benchmark import _build_our_model
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
    model = _build_our_model(h.getLp()); del h
    print("BUILT", flush=True)
    t0 = time.perf_counter()
    out = model.solve(root_solver="auto", distinguish_infeasible_unbounded=True, threads=threads, time_limit=limit)
    t = time.perf_counter() - t0
    print("RESULT " + json.dumps({"time": t, "status": out["status"], "obj": out["objective"],
                                  "events": [[n, s] for n, s in _core.last_solve_events()]}), flush=True)


def worker_highs(mps: str, mode: str, threads: int, limit: float) -> None:
    import highspy
    S = highspy.HighsModelStatus
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
    h.setOptionValue("threads", threads); h.setOptionValue("time_limit", limit)
    if mode == "simplex":
        h.setOptionValue("solver", "simplex"); h.setOptionValue("parallel", "on")
    else:
        h.setOptionValue("solver", "ipx"); h.setOptionValue("run_crossover", "on")
    print("READY", flush=True)
    sys.stdin.readline()
    t0 = time.perf_counter(); h.run(); t = time.perf_counter() - t0
    st = {S.kOptimal: "optimal", S.kInfeasible: "infeasible", S.kUnbounded: "unbounded",
          S.kTimeLimit: "timeout"}.get(h.getModelStatus(), str(h.getModelStatus()).split(".")[-1])
    info = h.getInfo()
    print("RESULT " + json.dumps({"time": t, "status": st, "obj": info.objective_function_value if st == "optimal" else None,
                                  "simplex_iters": info.simplex_iteration_count, "ipm_iters": info.ipm_iteration_count}), flush=True)


# ---------------------------------------------------------------- 親

class MemWatch:
    """子プロセス (とその子) の RSS の合計が mem_gb を超えたら全部止める。"""

    def __init__(self, procs, mem_gb):
        self.procs, self.limit, self.peak, self.tripped = procs, mem_gb * 2**30, 0, False
        self._stop = threading.Event()
        threading.Thread(target=self._run, daemon=True).start()

    def _run(self):
        import psutil
        while not self._stop.wait(0.5):
            tot = 0
            for p in self.procs:
                try:
                    pp = psutil.Process(p.pid)
                    for q in [pp] + pp.children(recursive=True):
                        tot += q.memory_info().rss
                except Exception:  # noqa: BLE001
                    pass
            self.peak = max(self.peak, tot)
            if tot > self.limit:
                self.tripped = True
                for p in self.procs:
                    kill_tree(p)
                return

    def stop(self):
        self._stop.set()


def kill_tree(p):
    import psutil
    try:
        pp = psutil.Process(p.pid)
        for q in pp.children(recursive=True) + [pp]:
            try: q.kill()
            except Exception: pass  # noqa: E701
    except Exception:  # noqa: BLE001
        pass


def _reader(name, p, q):
    for line in p.stdout:
        q.put((name, line.rstrip("\n")))
    q.put((name, None))


def run_enomoto(mps: Path, a) -> dict:
    env = dict(os.environ, RAYON_NUM_THREADS=str(a.threads))
    p = subprocess.Popen([sys.executable, __file__, "--worker-enomoto", str(mps), str(a.threads), str(a.time_limit)],
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
    q: queue.Queue = queue.Queue()
    threading.Thread(target=_reader, args=("e", p, q), daemon=True).start()
    err: list[str] = []
    threading.Thread(target=lambda: err.append(p.stderr.read()), daemon=True).start()
    mw = MemWatch([p], a.mem_gb)
    res = None
    try:
        deadline = time.time() + a.read_timeout
        built = False
        while True:
            try:
                _, line = q.get(timeout=max(0.1, deadline - time.time()))
            except queue.Empty:
                res = {"status": "timeout" if built else "build_timeout", "time": a.time_limit}
                break
            if line is None:
                break
            if line == "BUILT":
                built = True; deadline = time.time() + a.time_limit + 120
            elif line.startswith("RESULT "):
                res = json.loads(line[7:]); break
    finally:
        kill_tree(p); mw.stop()
    if res is None:
        p.wait(timeout=30)
        res = {"status": "memout" if mw.tripped else "crash", "time": a.time_limit, "rc": p.returncode,
               "stderr": (err[0] if err else "")[-1500:]}
    elif res["status"] == "time_limit" or res["time"] > a.time_limit:
        res["status"] = "timeout"
    res["peak_rss_gb"] = round(mw.peak / 2**30, 2)
    return res


def run_highs_race(mps: Path, a) -> dict:
    q: queue.Queue = queue.Queue()
    procs = {}
    for mode in ("simplex", "ipx"):
        p = subprocess.Popen([sys.executable, __file__, "--worker-highs", str(mps), mode, str(a.threads), str(a.time_limit)],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        procs[mode] = p
        threading.Thread(target=_reader, args=(mode, p, q), daemon=True).start()
    mw = MemWatch(list(procs.values()), a.mem_gb)
    ready, results, dead = set(), {}, set()
    out = None
    try:
        deadline = time.time() + a.read_timeout
        while len(ready | dead) < 2:
            name, line = q.get(timeout=max(0.1, deadline - time.time()))
            if line is None: dead.add(name)
            elif line == "READY": ready.add(name)
        for name in ready:
            procs[name].stdin.write("GO\n"); procs[name].stdin.flush()
        deadline = time.time() + a.time_limit + 60
        while len(results) + len(dead - set(results)) < 2:
            name, line = q.get(timeout=max(0.1, deadline - time.time()))
            if line is None:
                dead.add(name); continue
            if line.startswith("RESULT "):
                r = json.loads(line[7:]); results[name] = r
                if r["status"] in CONCLUSIVE:
                    out = dict(r, winner=name, other={k: v for k, v in results.items() if k != name}); break
    except queue.Empty:
        pass
    finally:
        for p in procs.values():
            kill_tree(p)
        mw.stop()
    if out is None:
        st = "memout" if mw.tripped else ("timeout" if not results or all(r["status"] == "timeout" for r in results.values()) else "fail")
        out = {"status": st, "time": a.time_limit, "detail": results, "dead": sorted(dead)}
    elif out["time"] > a.time_limit:
        out["status"] = "timeout"
    out["peak_rss_gb"] = round(mw.peak / 2**30, 2)
    return out


def wsl_path(p: Path) -> str:
    s = str(p.resolve()).replace("\\", "/")
    return "/mnt/" + s[0].lower() + s[2:]


def run_clp(mps: Path, a) -> dict:
    cap = int(a.time_limit + a.read_timeout)
    cmd = ["wsl", "-e", "timeout", "-s", "KILL", str(cap), CLP_BIN, wsl_path(mps), str(a.time_limit)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=cap + 60)
        out, rc, err = p.stdout, p.returncode, p.stderr
    except subprocess.TimeoutExpired:
        out, rc, err = "", -1, "parent timeout"
    finally:
        subprocess.run(["wsl", "-u", "root", "-e", "sh", "-c", "pkill -9 -f clp_driver; sync; echo 3 > /proc/sys/vm/drop_caches"],
                       capture_output=True)
    res = None
    for line in out.splitlines():
        if line.startswith("{") and '"built"' not in line:
            res = json.loads(line)
    if res is None:
        return {"status": "timeout" if rc in (-1, 137) else "crash", "time": a.time_limit, "rc": rc, "stderr": err[-1500:]}
    if res["status"] in ("limit", "stopped") or res.get("time", 0) > a.time_limit:
        res["status"] = "timeout"
    if res["status"] != "optimal":
        res["obj"] = None
    return res


RUNNERS = {"enomoto": run_enomoto, "highs": run_highs_race, "clp": run_clp}


# ---------------------------------------------------------------- データ

def free_gb() -> float:
    return shutil.disk_usage(REPO).free / 2**30


def prefetch(order, a, log):
    for name, rel, *_ in order:
        raw = CACHE / "raw" / rel.replace("/", "__")
        if raw.exists():
            continue
        try:
            import urllib.request
            size = int(urllib.request.urlopen(urllib.request.Request(BASE_URL + rel, method="HEAD"), timeout=60)
                       .headers.get("Content-Length", 0)) / 2**30
        except Exception as e:  # noqa: BLE001
            log(f"prefetch {name}: HEAD failed ({e})"); continue
        if free_gb() - size < a.min_free_gb + a.prefetch_reserve_gb:
            log(f"prefetch {name}: skipped (disk {free_gb():.1f} GB free, file {size:.2f} GB)"); continue
        try:
            ensure_raw(CACHE, name, rel); log(f"prefetch {name}: ok ({size*1024:.0f} MB)")
        except Exception as e:  # noqa: BLE001
            log(f"prefetch {name}: failed ({e})")


def load_ref():
    import csv
    ref = {}
    f = REPO / "benchmarks/paper/per_problem.csv"
    if f.exists():
        for r in csv.DictReader(open(f, encoding="utf-8")):
            if r.get("set") == "mittelmann" and r.get("reference_obj"):
                ref[r["problem"]] = float(r["reference_obj"])
    return ref


# ---------------------------------------------------------------- 要約

def summarize(out: Path, solvers, limit):
    recs = [json.loads(l) for l in out.read_text(encoding="utf-8").splitlines() if l.strip()]
    meta = next((r for r in recs if r.get("type") == "meta"), {})
    probs: dict = {}
    for r in recs:
        if r.get("type") == "run":
            probs.setdefault(r["problem"], {"runs": {}, "done": False})["runs"].setdefault(r["solver"], []).append(r["result"])
        elif r.get("type") == "done":
            probs.setdefault(r["problem"], {"runs": {}, "done": False})["done"] = True
        elif r.get("type") == "skip":
            probs[r["problem"]] = {"runs": {}, "done": True, "skip": r["reason"]}
    ref = load_ref()
    order = [p[0] for p in sorted(PROBLEMS, key=lambda p: p[4]) if p[0] in probs]

    def refobj(name, runs):
        if name in ref:
            return ref[name]
        objs = [runs[s][0]["obj"] for s in ("highs", "clp", "enomoto") if s in runs and runs[s][0]["status"] == "optimal"
                and runs[s][0].get("obj") is not None]
        return objs[0] if objs else None

    def solved(name, runs, s):
        if s not in runs or runs[s][0]["status"] != "optimal":
            return False
        t = refobj(name, runs)
        o = runs[s][0].get("obj")
        return t is None or (o is not None and abs(o - t) <= 1e-6 * max(1.0, abs(t)))

    def eff(name, runs, s):
        if not solved(name, runs, s):
            return limit
        return min(limit, statistics.median(x["time"] for x in runs[s]))

    done = [n for n in order if probs[n]["done"] and "skip" not in probs[n]]
    L = [f"# Mittelmann LPopt 全問: ENOMOTO auto vs HiGHS 疑似 auto vs CLP\n",
         f"- 開始: {meta.get('started')}、commit {meta.get('commit')}、制限時間 {limit:.0f} 秒、threads={meta.get('threads')}"
         f" (CLP は逐次)、メモリ上限 {meta.get('mem_gb')} GB (超えたら memout)",
         f"- 環境: {meta.get('cpu')}、Windows {meta.get('os')}、HiGHS {meta.get('highs')}、CLP 1.17.11 (WSL2 Ubuntu、論文用ドライバ)",
         f"- 終わった問題: {len(done)} / {len(PROBLEMS)}" + (f" (飛ばした問題 {sum('skip' in v for v in probs.values())})" if any('skip' in v for v in probs.values()) else ""),
         "- 解けた = optimal かつ目的値が基準値 (論文の計測の基準値、無ければ HiGHS→CLP→ENOMOTO の順の最適値) と相対 1e-6 以内。"
         "解けなかった問題は制限時間として集計。30 秒未満の問題は 3 回の中央値\n"]
    if done:
        L.append("| | " + " | ".join(solvers) + " |")
        L.append("|---|" + "---|" * len(solvers))
        L.append("| 解けた数 | " + " | ".join(str(sum(solved(n, probs[n]["runs"], s) for n in done)) for s in solvers) + " |")
        sgm = {s: math.exp(sum(math.log(eff(n, probs[n]["runs"], s) + SHIFT) for n in done) / len(done)) - SHIFT for s in solvers}
        L.append("| 10 秒シフト幾何平均 (秒) | " + " | ".join(f"{sgm[s]:.2f}" for s in solvers) + " |")
        if "highs" in sgm:
            L.append("| 同 HiGHS 比 | " + " | ".join(f"{sgm[s]/sgm['highs']:.2f}x" for s in solvers) + " |")
        L.append("| 総時間 (秒) | " + " | ".join(f"{sum(eff(n, probs[n]['runs'], s) for n in done):.0f}" for s in solvers) + " |")
    L.append("\n## 問題ごと (秒。解けなかったら状態)\n")
    L.append("| 問題 | 行 | 列 | nnz | " + " | ".join(solvers) + " | HiGHS の勝者 | ENOMOTO の race |")
    L.append("|---|---|---|---|" + "---|" * len(solvers) + "---|---|")
    size = {p[0]: p[2:] for p in PROBLEMS}
    for n in order:
        v = probs[n]
        r, c, z = size[n]
        if "skip" in v:
            L.append(f"| {n} | {r} | {c} | {z} | " + " | ".join(["skip"] * len(solvers)) + f" | | {v['skip']} |"); continue
        cells = []
        for s in solvers:
            if s not in v["runs"]:
                cells.append("…"); continue
            x = v["runs"][s][0]
            if solved(n, v["runs"], s):
                cells.append(f"{statistics.median(y['time'] for y in v['runs'][s]):.2f}")
            else:
                cells.append(x["status"] if x["status"] != "optimal" else f"wrong obj {x.get('obj'):.10g}")
        hw = v["runs"].get("highs", [{}])[0].get("winner", "")
        ev = [e[0] for e in v["runs"].get("enomoto", [{}])[0].get("events", []) if e[0].startswith("race_")]
        L.append(f"| {n}{'' if v['done'] else ' (途中)'} | {r} | {c} | {z} | " + " | ".join(cells) + f" | {hw} | {','.join(ev)} |")
    out.with_suffix(".md").write_text("\n".join(L) + "\n", encoding="utf-8")


# ---------------------------------------------------------------- main

def keep_awake():
    if sys.platform == "win32":
        import ctypes
        ctypes.windll.kernel32.SetThreadExecutionState(0x80000000 | 0x00000001)  # ES_CONTINUOUS | ES_SYSTEM_REQUIRED


def main():
    if sys.argv[1:2] == ["--worker-enomoto"]:
        return worker_enomoto(sys.argv[2], int(sys.argv[3]), float(sys.argv[4]))
    if sys.argv[1:2] == ["--worker-highs"]:
        return worker_highs(sys.argv[2], sys.argv[3], int(sys.argv[4]), float(sys.argv[5]))
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--time-limit", type=float, default=600.0)
    ap.add_argument("--threads", type=int, default=16)
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--single-run-above", type=float, default=30.0)
    ap.add_argument("--read-timeout", type=float, default=1800.0, help="読み込み・モデル構築の上限 (秒、計時しない)")
    ap.add_argument("--mem-gb", type=float, default=13.0)
    ap.add_argument("--min-free-gb", type=float, default=1.0, help="MPS の展開後にディスクに残すべき空き")
    ap.add_argument("--prefetch-reserve-gb", type=float, default=1.0, help="先取りの後に展開用に残す空き")
    ap.add_argument("--solvers", nargs="+", default=["enomoto", "highs", "clp"], choices=list(RUNNERS))
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--commit", default="")
    a = ap.parse_args()
    keep_awake()

    def log(msg):
        print(f"[{datetime.now():%H:%M:%S}] {msg}", flush=True)

    def append(rec):
        with open(a.out, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec) + "\n"); f.flush(); os.fsync(f.fileno())

    done = set()
    if a.out.exists():
        for l in a.out.read_text(encoding="utf-8").splitlines():
            r = json.loads(l)
            # ディスク不足で飛ばした問題は再開時にもう一度試す (空きを作ってから同じコマンドで再実行)
            if r.get("type") == "done" or (r.get("type") == "skip" and not r["reason"].startswith("disk")):
                done.add(r["problem"])
    else:
        import highspy
        append({"type": "meta", "started": datetime.now(timezone.utc).isoformat(timespec="seconds"), "commit": a.commit,
                "threads": a.threads, "time_limit": a.time_limit, "mem_gb": a.mem_gb, "solvers": a.solvers,
                "cpu": platform.processor(), "os": platform.version(),
                "highs": f"{highspy.HIGHS_VERSION_MAJOR}.{highspy.HIGHS_VERSION_MINOR}.{highspy.HIGHS_VERSION_PATCH}"
                if hasattr(highspy, "HIGHS_VERSION_MAJOR") else "1.15.1"})

    order = sorted(PROBLEMS, key=lambda p: p[4])
    if a.only:
        order = [p for p in order if p[0] in a.only]
    prefetch([p for p in order if p[0] not in done], a, log)

    for name, rel, rows, cols, nnz in order:
        if name in done:
            continue
        (CACHE / "tmp").mkdir(parents=True, exist_ok=True)
        wd = Path(tempfile.mkdtemp(dir=CACHE / "tmp"))
        try:
            try:
                raw = ensure_raw(CACHE, name, rel)
            except Exception as e:  # noqa: BLE001
                log(f"{name}: download failed ({e}), 後で再実行すれば取りにいく"); continue
            need = raw.stat().st_size * 12 / 2**30
            if free_gb() - need < a.min_free_gb:
                append({"type": "skip", "problem": name, "reason": f"disk ({free_gb():.1f} GB free, MPS ~{need:.1f} GB)"})
                log(f"{name}: skipped, disk"); summarize(a.out, a.solvers, a.time_limit); continue
            try:
                mps = materialize_mps(CACHE, rel, raw, wd)
            except Exception as e:  # noqa: BLE001
                append({"type": "skip", "problem": name, "reason": f"materialize failed: {e}"[:300]})
                log(f"{name}: materialize failed {e}"); summarize(a.out, a.solvers, a.time_limit); continue
            first = {}
            for rep in range(a.reps):
                k = len(a.solvers)
                ordr = a.solvers[rep % k:] + a.solvers[:rep % k]
                for s in ordr:
                    t0 = time.time()
                    r = RUNNERS[s](mps, a)
                    append({"type": "run", "problem": name, "solver": s, "rep": rep, "result": r,
                            "at": datetime.now(timezone.utc).isoformat(timespec="seconds")})
                    log(f"{name:22s} rep{rep} {s:8s} {r['status']:10s} {r.get('time', 0):9.3f}s obj={r.get('obj')} (wall {time.time()-t0:.0f}s)")
                    if rep == 0: first[s] = r
                if rep == 0 and (max(x["time"] for x in first.values()) >= a.single_run_above
                                 or any(x["status"] not in CONCLUSIVE for x in first.values())):
                    break
            append({"type": "done", "problem": name})
            summarize(a.out, a.solvers, a.time_limit)
        finally:
            shutil.rmtree(wd, ignore_errors=True)
    summarize(a.out, a.solvers, a.time_limit)
    log("all done")


if __name__ == "__main__":
    main()
