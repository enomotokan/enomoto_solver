"""論文用ベンチマーク: enomoto_solver と HiGHS・CLP・SoPlex の求解時間の比較。

対象 (データは prepare_data.py で用意する):
  netlib       Netlib の有限最適解をもつ 93 問 (期待: optimal)
  infeas       Netlib の実行不能問題 29 問 (期待: infeasible)
  infeas_dual  その双対 29 問 (期待: unbounded。cplex2_dual だけは期待状態を定めない)
  kennington   Kennington の 16 問 (期待: optimal)
  mittelmann   Mittelmann LPopt の公開問題 (期待: optimal。scripts/run_mittelmann_benchmark.py の一覧)

計測の決まり:
  - 1 回の求解ごとに新しいプロセスで解く。MPS の読み込み (enomoto はモデル構築も) は計時しない。
    時間は各ソルバーの求解呼び出しの前後の実時間 (enomoto: Model.solve()、HiGHS: Highs.run()、
    CLP: ClpSimplex::initialSolve()、SoPlex: SoPlex::optimize())。
  - 設定はどのソルバーも既定値 (時間制限だけ設定)。enomoto は distinguish_infeasible_unbounded=True
    (実行不能と非有界を区別する) で解く。並列計算の効果も含めて測るため、HiGHS/enomoto のスレッド数は
    既定で論理 CPU 数 (--threads で変更、0 なら各ソルバーの既定値)。CLP/SoPlex は逐次。
  - 制限時間は --time-limit (既定 3600 秒)。enomoto には制限時間の設定が無いので、制限時間を
    過ぎたら親が止める。他のソルバーも、制限時間を大きく過ぎたら親が止める。
  - 各問題・各ソルバーを --reps 回 (既定 3) 解いて中央値を取る。1 回目が --single-run-above 秒
    (既定 600) 以上かかった、または解けなかった (制限時間・メモリ不足・異常終了) ときは 1 回だけ。
    その揺らぎの目安は、3 回測れた問題の時間帯ごとのばらつきと、--variability-probe で
    指定した問題の追加計測で示す。
  - 繰り返しの間でソルバーの順番を回す (ドリフトが特定のソルバーに偏らないように)。
  - 解けた = 期待どおりの状態で、optimal なら目的関数値が基準値と相対 --obj-rtol (既定 1e-6) 以内。
    基準値は、最適と答えたソルバーの値のうち最も多くのソルバーと一致するもの (同数なら HiGHS 優先)。
    中央値を取る前に、解けなかった回は制限時間として扱う。
  - 集計: 幾何平均・10 秒シフト付き幾何平均・総時間 (いずれも解けなかった問題は制限時間として含める)、
    および全ソルバーが解けた問題だけの幾何平均。
  - infeas / infeas_dual では enomoto の節目の時刻 (_core.last_solve_events()、求解の開始 =
    前処理の直前から) も記録する: 段階 A の終わりで z^1 < 0 (実行不能か非有界) を検出した時刻と、
    段階 B の後に実行不能/非有界を結論した時刻 (単体法の終了時。前処理が結論したらその時刻)。

結果は --out-dir (既定 benchmarks/paper/) に書く:
  results.json    全計測の生データ (1 回の求解ごとに追記。同じコマンドで再開できる)
  summary.md      集計表・問題ごとの表・環境 (CPU の型番、ソルバーの版)
  per_problem.csv 問題 × ソルバーごとの中央値など

使い方 (リポジトリのルートで、Linux。準備は README.md 参照):
  .paper_venv/bin/python scripts/paper_bench/run_paper_bench.py                # 全部
  .paper_venv/bin/python scripts/paper_bench/run_paper_bench.py --sets netlib  # 一部の集合
  .paper_venv/bin/python scripts/paper_bench/run_paper_bench.py --shard 0/4    # 4 台に分ける 1 台目
  .paper_venv/bin/python scripts/paper_bench/run_paper_bench.py --report-only --inputs a.json b.json
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

SOLVERS = ["enomoto", "highs", "clp", "soplex"]
SOLVER_LABEL = {"enomoto": "ENOMOTO", "highs": "HiGHS", "clp": "CLP", "soplex": "SoPlex"}
SETS = ["netlib", "infeas", "infeas_dual", "kennington", "mittelmann"]
SET_LABEL = {
    "netlib": "Netlib (有限最適解あり)",
    "infeas": "Netlib 実行不能",
    "infeas_dual": "Netlib 実行不能の双対 (非有界)",
    "kennington": "Kennington",
    "mittelmann": "Mittelmann LPopt",
}
SHIFT = 10.0
# 解けたとみなしうる終了状態 (期待状態を定めない問題用)。
DEFINITIVE = {"optimal", "infeasible", "unbounded", "infeasible_or_unbounded"}
# enomoto の節目のうち、最終的な結論 (実行不能/非有界) を出したもの。
VERDICT_EVENTS = ("presolve_infeasible", "presolve_no_finite_optimum", "stage_b_infeasible",
                  "finish_unbounded", "polish_unbounded", "trivial_unbounded")


# ================================================================ 問題の一覧

@dataclass
class Problem:
    set: str
    name: str
    expected: str | None  # None: 期待状態を定めない
    path: Path | None  # 展開済みの MPS (mittelmann は None で、計測時に展開する)
    mittelmann_rel: str | None
    size: int  # 並べ替え用 (MPS の大きさ、mittelmann は非零数)

    @property
    def key(self) -> str:
        return f"{self.set}/{self.name}"


def _names(cache: Path) -> list[str]:
    return [l.strip() for l in (cache / "problems.txt").read_text().splitlines() if l.strip()]


def list_problems(sets: list[str]) -> list[Problem]:
    from prepare_data import CACHES

    out: list[Problem] = []
    expected_of = {"netlib": "optimal", "infeas": "infeasible", "infeas_dual": "unbounded", "kennington": "optimal"}
    for s in sets:
        if s == "mittelmann":
            from run_mittelmann_benchmark import PROBLEMS

            for name, rel, _rows, _cols, nnz in PROBLEMS:
                out.append(Problem(s, name, "optimal", None, rel, nnz))
            continue
        cache = CACHES[s]
        if not (cache / "problems.txt").exists():
            raise SystemExit(f"{cache}/problems.txt が無い: python scripts/paper_bench/prepare_data.py --sets {s}")
        for name in _names(cache):
            mps = cache / "mps" / f"{name}.mps"
            if not mps.exists():
                raise SystemExit(f"{mps} が無い: prepare_data.py を実行すること")
            expected = expected_of[s]
            if name == "cplex2_dual":
                expected = None  # prepare_data.py の説明を参照
            out.append(Problem(s, name, expected, mps, None, mps.stat().st_size))
    return out


# ================================================================ ワーカー (子プロセス)

def _emit(obj: dict) -> None:
    print(json.dumps(obj), flush=True)


def _build_our_model(lp):
    """HiGHS の LP から enomoto のモデルを作る (scripts/run_mittelmann_benchmark.py と同じ。
    pybind のベクトルは添字アクセスのたびに全体を写すので、先に 1 回だけリストにする)。"""
    from enomoto_solver import _core

    m = _core.PyModel()
    n = lp.num_col_
    col_lower, col_upper = list(lp.col_lower_), list(lp.col_upper_)
    for j in range(n):
        lb, ub = float(col_lower[j]), float(col_upper[j])
        m.add_variable("continuous", lb, ub)
    obj_coeffs = [(j, float(c)) for j, c in enumerate(list(lp.col_cost_)) if c != 0.0]
    sense = "maximize" if "kMaximize" in str(lp.sense_) else "minimize"
    m.set_objective(obj_coeffs, float(lp.offset_), sense)

    n_rows = lp.num_row_
    rows: list = [[] for _ in range(n_rows)]
    am = lp.a_matrix_
    start, index, value = list(am.start_), list(am.index_), list(am.value_)
    if "kColwise" in str(am.format_):
        for j in range(n):
            for k in range(start[j], start[j + 1]):
                if value[k] != 0.0:
                    rows[index[k]].append((j, value[k]))
    else:
        for i in range(n_rows):
            for k in range(start[i], start[i + 1]):
                if value[k] != 0.0:
                    rows[i].append((index[k], value[k]))
    del start, index, value
    row_lower, row_upper = list(lp.row_lower_), list(lp.row_upper_)
    for i in range(n_rows):
        lo, hi = row_lower[i], row_upper[i]
        terms = rows[i]
        rows[i] = None
        if math.isinf(lo) and math.isinf(hi):
            continue
        if not math.isinf(lo) and abs(hi - lo) < 1e-12:
            m.add_constraint(terms, "==", float(lo))
            continue
        if not math.isinf(hi):
            m.add_constraint(terms, "<=", float(hi))
        if not math.isinf(lo):
            m.add_constraint(terms, ">=", float(lo))
    return m


def _read_highs(mps: str):
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    status = h.readModel(mps)
    if "kOk" not in str(status) and "kWarning" not in str(status):
        _emit({"status": "read_error", "detail": str(status)})
        return None
    return h


def worker_enomoto(mps: str) -> None:
    from enomoto_solver import _core

    t0 = time.perf_counter()
    h = _read_highs(mps)
    if h is None:
        return
    model = _build_our_model(h.getLp())
    del h
    _emit({"built": True, "read_time": time.perf_counter() - t0})
    t0 = time.perf_counter()
    out = model.solve(root_solver=None, distinguish_infeasible_unbounded=True)
    t = time.perf_counter() - t0
    _emit({"time": t, "status": out["status"], "obj": out.get("objective"),
           "events": [[name, sec] for name, sec in _core.last_solve_events()]})


HIGHS_STATUS = {
    "kOptimal": "optimal",
    "kInfeasible": "infeasible",
    "kUnbounded": "unbounded",
    "kUnboundedOrInfeasible": "infeasible_or_unbounded",
    "kTimeLimit": "limit",
    "kIterationLimit": "limit",
}


def worker_highs(mps: str, time_limit: float, threads: int) -> None:
    t0 = time.perf_counter()
    h = _read_highs(mps)
    if h is None:
        return
    h.setOptionValue("time_limit", float(time_limit))
    if threads > 0:
        h.setOptionValue("threads", int(threads))
    _emit({"built": True, "read_time": time.perf_counter() - t0})
    t0 = time.perf_counter()
    h.run()
    t = time.perf_counter() - t0
    raw = str(h.getModelStatus()).split(".")[-1]
    st = HIGHS_STATUS.get(raw, "other")
    info = h.getInfo()
    _emit({"time": t, "status": st, "raw_status": raw,
           "obj": h.getObjectiveValue() if st == "optimal" else None,
           "iters": int(info.simplex_iteration_count), "ipm_iters": int(info.ipm_iteration_count)})


# ================================================================ 1 回の求解 (親プロセス)

def _mem_limiter(mem_gb: float):
    def _set() -> None:
        import resource

        lim = int(mem_gb * (1 << 30))
        resource.setrlimit(resource.RLIMIT_AS, (lim, lim))
    return _set


def _classify_crash(rc: int, stderr: str) -> str:
    if "MemoryError" in stderr or "memory allocation" in stderr or "bad_alloc" in stderr or rc in (-6, -9):
        return "memory"
    return "crash"


def run_once(solver: str, mps: Path, args) -> dict:
    """`solver` で `mps` を新しいプロセスで 1 回解き、結果の dict を返す。"""
    limit = args.time_limit
    env = os.environ.copy()
    if solver == "enomoto":
        cmd = [sys.executable, __file__, "--worker", "enomoto", str(mps)]
        if args.threads > 0:
            env["RAYON_NUM_THREADS"] = str(args.threads)
        grace = 5.0  # 制限時間の設定が無いので、制限時間を過ぎたら止める
    elif solver == "highs":
        cmd = [sys.executable, __file__, "--worker", "highs", str(mps), "--time-limit", str(limit),
               "--threads", str(args.threads)]
        grace = 60.0 + 0.05 * limit
    else:
        cmd = [str(args.clp_bin if solver == "clp" else args.soplex_bin), str(mps), str(limit)]
        grace = 60.0 + 0.05 * limit
    popen_kw: dict = {}
    if os.name == "posix" and args.mem_gb > 0:
        popen_kw["preexec_fn"] = _mem_limiter(args.mem_gb)

    started = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env, **popen_kw)
    lines: list[str] = []
    err: list[str] = []
    built = threading.Event()

    def _read_out() -> None:
        for line in p.stdout:  # type: ignore[union-attr]
            lines.append(line.strip())
            if '"built"' in line:
                built.set()

    th_out = threading.Thread(target=_read_out, daemon=True)
    th_err = threading.Thread(target=lambda: err.append(p.stderr.read()), daemon=True)  # type: ignore[union-attr]
    th_out.start()
    th_err.start()

    t_start = time.time()
    while not built.is_set() and p.poll() is None and time.time() - t_start < args.read_timeout:
        time.sleep(0.05)
    if not built.is_set() and p.poll() is None:
        p.kill()
        p.wait()
        return {"status": "read_timeout", "started": started}
    try:
        p.wait(timeout=limit + grace)
    except subprocess.TimeoutExpired:
        p.kill()
        p.wait()
        th_out.join(5)
        return {"status": "timeout", "started": started, "killed": True}
    th_out.join(5)
    th_err.join(5)
    stderr = "".join(err)
    results = [json.loads(l) for l in lines if l.startswith("{") and '"built"' not in l]
    built_info = next((json.loads(l) for l in lines if '"built"' in l), {})
    if p.returncode != 0 or not results:
        return {"status": _classify_crash(p.returncode, stderr), "returncode": p.returncode,
                "stderr": stderr[-800:], "started": started}
    r = results[-1]
    r["started"] = started
    r.setdefault("read_time", built_info.get("read_time"))
    if r.get("time") is not None and r["time"] > limit and r.get("status") in DEFINITIVE:
        r["status"] = "limit"  # 制限時間を過ぎてから出た答えは数えない
    return r


# ================================================================ 環境の記録

def _read(path: str) -> str | None:
    try:
        return Path(path).read_text().strip()
    except OSError:
        return None


def _cmd(cmd: list[str]) -> str | None:
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=60).stdout.strip() or None
    except (OSError, subprocess.SubprocessError):
        return None


def machine_info(args) -> dict:
    cpu = None
    cpuinfo = _read("/proc/cpuinfo")
    if cpuinfo:
        cpu = next((l.split(":", 1)[1].strip() for l in cpuinfo.splitlines() if l.startswith("model name")), None)
    ram_gb = None
    if hasattr(os, "sysconf"):
        try:
            ram_gb = os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES") / 2**30
        except (ValueError, OSError):
            pass
    git_head = _cmd(["git", "-C", str(REPO), "rev-parse", "HEAD"])
    git_dirty = _cmd(["git", "-C", str(REPO), "status", "--porcelain", "--untracked-files=no"])
    highs_version = _cmd([sys.executable, "-c", "import highspy; print(highspy.Highs().version())"])
    enomoto_env = {k: v for k, v in os.environ.items() if k.startswith("ENOMOTO_") or k.startswith("RAYON_")}
    return {
        "hostname": platform.node(),
        "recorded_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "cpu_model": cpu or platform.processor() or None,
        "logical_cpus": os.cpu_count(),
        "lscpu": _cmd(["lscpu"]),
        "ram_gb": ram_gb,
        "os": platform.platform(),
        "dmi_vendor": _read("/sys/class/dmi/id/sys_vendor"),
        "dmi_product": _read("/sys/class/dmi/id/product_name"),
        "cpu_governor": _read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
        "python": sys.version.split()[0],
        # setup_solvers.sh は PATH を変えずに rustup で入れるので、$CARGO_HOME/bin も見る。
        "rustc": _cmd(["rustc", "--version"])
        or _cmd([str(Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "bin" / "rustc"), "--version"]),
        "versions": {
            "enomoto": f"git {git_head[:12] if git_head else '?'}{' (未コミットの変更あり)' if git_dirty else ''}",
            "highs": highs_version,
            "clp": _cmd([str(args.clp_bin), "--version"]),
            "soplex": _cmd([str(args.soplex_bin), "--version"]),
        },
        "env_overrides": enomoto_env,
    }


# ================================================================ 計測の本体

def _load(path: Path) -> dict:
    if path.exists():
        return json.loads(path.read_text())
    return {"settings": {}, "environments": [], "runs": {}, "variability": {}, "warmup": {}}


def _save(data: dict, path: Path) -> None:
    tmp = path.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(data, indent=1, sort_keys=True))
    tmp.replace(path)


def _needs_more(runs: list[dict], reps: int, args) -> bool:
    if len(runs) >= reps:
        return False
    if not runs:
        return True
    first = runs[0]
    if first.get("status") not in DEFINITIVE:
        return False  # 制限時間・メモリ不足・異常終了: 繰り返さない
    return (first.get("time") or 0.0) < args.single_run_above


class Materialized:
    """問題の MPS のパス。mittelmann は一時ディレクトリに展開し、終わったら消す。"""

    def __init__(self, prob: Problem):
        self.prob = prob
        self.workdir: Path | None = None

    def __enter__(self) -> Path:
        if self.prob.path is not None:
            return self.prob.path
        from prepare_data import CACHES
        from run_mittelmann_benchmark import ensure_raw, materialize_mps

        cache = CACHES["mittelmann"]
        raw = ensure_raw(cache, self.prob.name, self.prob.mittelmann_rel)
        self.workdir = Path(tempfile.mkdtemp(dir=cache))
        return materialize_mps(cache, self.prob.mittelmann_rel, raw, self.workdir)

    def __exit__(self, *exc) -> None:
        if self.workdir is not None:
            shutil.rmtree(self.workdir, ignore_errors=True)


def rewrite_mps(mps: Path, out_dir: Path) -> Path:
    """HiGHS で読み直し、行・列の名前を付け直した MPS を書く。固定形式の MPS で名前に空白を含む問題
    (Netlib の forplan など) を CLP/SoPlex が読めないときに使う。問題そのものは変わらない。"""
    import highspy

    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / f"{mps.stem}.renamed.mps"
    if out.exists():
        return out
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(str(mps))
    lp = h.getLp()
    lp.col_names_ = [f"c{j}" for j in range(lp.num_col_)]
    lp.row_names_ = [f"r{i}" for i in range(lp.num_row_)]
    w = highspy.Highs()
    w.setOptionValue("output_flag", False)
    w.passModel(lp)
    w.writeModel(str(out))
    return out


def run_solver(s: str, mps: Path, args, prob: Problem) -> dict:
    """[`run_once`] に、CLP/SoPlex が MPS を読めなかったときの書き直し MPS での再試行を足したもの。"""
    r = run_once(s, mps, args)
    if r.get("status") == "read_error" and s in ("clp", "soplex"):
        renamed = rewrite_mps(mps, args.out_dir / "renamed_mps" / prob.set)
        r = run_once(s, renamed, args)
        r["renamed_mps"] = True
    return r


def run_benchmark(args) -> None:
    out_json = args.out_dir / "results.json"
    data = _load(out_json)
    data.setdefault("warmup", {})
    settings = {
        "time_limit_s": args.time_limit,
        "reps": args.reps,
        "single_run_above_s": args.single_run_above,
        "warmup_below_s": args.warmup_below,
        "threads": args.threads,
        "mem_gb": args.mem_gb,
        "obj_rtol": args.obj_rtol,
    }
    if data["settings"] and any(data["settings"].get(k) != v for k, v in settings.items() if k != "obj_rtol"):
        raise SystemExit(f"{out_json} は別の設定で計測されている: {data['settings']} (別の --out-dir を使うこと)")
    data["settings"] = settings
    if args.retry_statuses:
        # 指定した状態で終わった回があるソルバーは、その問題を最初から測り直す。
        for key, runs_of in data["runs"].items():
            for s, runs in runs_of.items():
                if any(r.get("status") in args.retry_statuses for r in runs):
                    print(f"retry: {key} {s} ({[r.get('status') for r in runs]})", flush=True)
                    runs.clear()
                    data["warmup"].get(key, {}).pop(s, None)
    env = machine_info(args)
    data["environments"].append(env)
    models = {e.get("cpu_model") for e in data["environments"]}
    if len(models) > 1:
        print(f"警告: 計測に使った CPU が複数ある: {models}", file=sys.stderr)
    _save(data, out_json)

    problems = list_problems(args.sets)
    if args.only:
        problems = [p for p in problems if p.name in args.only or p.key in args.only]
    if args.exclude:
        problems = [p for p in problems if p.name not in args.exclude and p.key not in args.exclude]
    problems.sort(key=lambda p: (SETS.index(p.set), p.size))
    if args.shard:
        i, n = (int(x) for x in args.shard.split("/"))
        problems = [p for k, p in enumerate(problems) if k % n == i]
    solvers = args.solvers
    print(f"{len(problems)} 問 × {len(solvers)} ソルバー、CPU: {env['cpu_model']}", flush=True)

    for prob in problems:
        runs_of = data["runs"].setdefault(prob.key, {})
        if not any(_needs_more(runs_of.get(s, []), args.reps, args) for s in solvers):
            continue
        t_prob = time.time()
        with Materialized(prob) as mps:
            for rep in range(args.reps):
                k = rep % len(solvers)
                for s in solvers[k:] + solvers[:k]:
                    runs = runs_of.setdefault(s, [])
                    if len(runs) != rep or not _needs_more(runs, args.reps, args):
                        continue
                    r = run_solver(s, mps, args, prob)
                    warm = data["warmup"].setdefault(prob.key, {}).setdefault(s, [])
                    if rep == 0 and not warm and r.get("status") in DEFINITIVE and (r.get("time") or 0.0) < args.warmup_below:
                        # 短い問題の 1 回目は遅く出やすい (新しい問題の初回) ので、捨てて測り直す。
                        warm.append(r)
                        r = run_solver(s, mps, args, prob)
                    r["rep"] = rep
                    runs.append(r)
                    _save(data, out_json)
                    t = r.get("time")
                    print(f"  {prob.key:32s} {s:8s} #{rep + 1} {r.get('status'):24s} "
                          f"{'-' if t is None else f'{t:.4f}s'}", flush=True)
        print(f"{prob.key} done [{time.time() - t_prob:.0f}s]", flush=True)

    for key in args.variability_probe or []:
        prob = next((p for p in list_problems(SETS) if p.key == key or p.name == key), None)
        if prob is None:
            print(f"--variability-probe: {key} が見つからない", file=sys.stderr)
            continue
        probe = data["variability"].setdefault(prob.key, {})
        with Materialized(prob) as mps:
            for rep in range(args.probe_reps):
                k = rep % len(solvers)
                for s in solvers[k:] + solvers[:k]:
                    runs = probe.setdefault(s, [])
                    if len(runs) > rep:
                        continue
                    r = run_solver(s, mps, args, prob)
                    r["rep"] = rep
                    runs.append(r)
                    _save(data, out_json)
                    print(f"  probe {prob.key:26s} {s:8s} #{rep + 1} {r.get('status')} {r.get('time')}", flush=True)


# ================================================================ 集計

def _obj_close(a: float, b: float, rtol: float) -> bool:
    return abs(a - b) <= rtol * max(1.0, abs(a), abs(b))


def reference_objective(runs_of: dict, rtol: float) -> float | None:
    """最適と答えたソルバーの目的関数値 (ソルバーごとに中央値) のうち、最も多くのソルバーと
    一致する値 (同数なら HiGHS、次いで SOLVERS の順)。"""
    vals: dict[str, float] = {}
    for s in SOLVERS:
        objs = [r["obj"] for r in runs_of.get(s, []) if r.get("status") == "optimal" and r.get("obj") is not None]
        if objs:
            vals[s] = statistics.median(objs)
    if not vals:
        return None
    order = ["highs"] + [s for s in SOLVERS if s != "highs"]
    best, best_n = None, -1
    for s in order:
        if s not in vals:
            continue
        n = sum(_obj_close(vals[s], v, rtol) for v in vals.values())
        if n > best_n:
            best, best_n = vals[s], n
    return best


def run_ok(r: dict, expected: str | None, ref: float | None, rtol: float) -> bool:
    st = r.get("status")
    if expected is None:
        return st in DEFINITIVE
    if st != expected:
        return False
    if expected == "optimal" and ref is not None:
        return r.get("obj") is not None and _obj_close(r["obj"], ref, rtol)
    return True


def enomoto_phase_times(r: dict) -> dict:
    """enomoto の 1 回の求解の節目から、段階 A の検出時刻と結論の時刻を取り出す。
    解き直し (`retry`) があれば、最後の解き直し以降の節目だけを見る (時刻は最初からの経過)。"""
    ev = r.get("events") or []
    last_retry = max((i for i, (n, _) in enumerate(ev) if n == "retry"), default=-1)
    ev = ev[last_retry + 1:]
    names = [n for n, _ in ev]
    t_a = next((t for n, t in ev if n == "stage_a_no_finite_optimum"), None)
    if t_a is not None:
        a_state = "detected"
    elif "stage_a_end" in names:
        a_state = "z1=0"
    elif "stage_a_skipped" in names:
        a_state = "skipped"
    elif names and names[0].startswith("presolve_") and names[0] != "presolve_end":
        a_state = "presolve"
    else:
        a_state = "none"
    source = next((n for n, _ in ev if n in VERDICT_EVENTS), None)
    t_verdict = None
    if source is not None:
        if source.startswith("presolve_"):
            t_verdict = next(t for n, t in ev if n == source)
        else:
            t_verdict = next((t for n, t in ev if n == "simplex_end"), None)
    return {"t_a": t_a, "a_state": a_state, "t_verdict": t_verdict, "verdict_source": source,
            "retried": last_retry >= 0}


@dataclass
class Cell:
    """問題 × ソルバーの集計値。"""
    n_runs: int
    status: str | None
    time: float  # 実効時間の中央値 (解けなかった回は制限時間)
    solved: bool
    run_times: list
    obj: float | None
    phase: dict | None  # enomoto の段階の時刻 (中央値)


def summarize_cell(runs: list[dict], expected, ref, rtol, limit) -> Cell | None:
    if not runs:
        return None
    eff = [r["time"] if run_ok(r, expected, ref, rtol) else limit for r in runs]
    med = statistics.median(eff)
    statuses = [r.get("status") for r in runs]
    status = max(set(statuses), key=lambda s: (statuses.count(s), -statuses.index(s)))
    objs = [r["obj"] for r in runs if r.get("status") == "optimal" and r.get("obj") is not None]
    phase = None
    if any("events" in r for r in runs):
        ok = [enomoto_phase_times(r) for r in runs if run_ok(r, expected, ref, rtol)]
        if ok:
            ta = [p["t_a"] for p in ok if p["t_a"] is not None]
            tv = [p["t_verdict"] for p in ok if p["t_verdict"] is not None]
            phase = {
                "t_a": statistics.median(ta) if ta else None,
                "t_verdict": statistics.median(tv) if tv else None,
                "a_state": ok[0]["a_state"],
                "verdict_source": ok[0]["verdict_source"],
                "retried": any(p["retried"] for p in ok),
            }
    return Cell(len(runs), status, med, med < limit, [r.get("time") for r in runs],
                statistics.median(objs) if objs else None, phase)


def gm(ts: list[float]) -> float:
    return math.exp(sum(math.log(max(t, 1e-6)) for t in ts) / len(ts)) if ts else float("nan")


def sgm(ts: list[float], shift: float = SHIFT) -> float:
    return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift if ts else float("nan")


def _f(t: float | None, digits: int = 3) -> str:
    if t is None or (isinstance(t, float) and math.isnan(t)):
        return "-"
    if t >= 100:
        return f"{t:.0f}"
    if t >= 10:
        return f"{t:.1f}"
    if t < 0.01:
        digits = max(digits, 4)  # ミリ秒未満の問題が 0.000 と出ないように
    return f"{t:.{digits}f}"


def build_report(data: dict, out_dir: Path, exclude: list[str] | None = None) -> str:
    st = data["settings"]
    limit, rtol = st["time_limit_s"], st.get("obj_rtol", 1e-6)
    all_problems = {p.key: p for p in list_problems_safe()}
    excluded = set(exclude or [])
    cells: dict[str, dict[str, Cell]] = {}
    refs: dict[str, float | None] = {}
    for key, runs_of in data["runs"].items():
        if key in excluded or key.split("/", 1)[1] in excluded:
            continue
        prob = all_problems.get(key)
        expected = prob.expected if prob else None
        refs[key] = reference_objective(runs_of, rtol) if expected == "optimal" else None
        cells[key] = {s: c for s in SOLVERS
                      if (c := summarize_cell(runs_of.get(s, []), expected, refs[key], rtol, limit)) is not None}
    solvers = [s for s in SOLVERS if any(s in c for c in cells.values())]

    L: list[str] = []
    L.append("# 求解時間の比較: ENOMOTO・HiGHS・CLP・SoPlex\n")
    L.append("## 計測環境と設定\n")
    envs = data["environments"]
    cpus = sorted({e.get("cpu_model") or "?" for e in envs})
    L.append(f"- CPU: {' / '.join(cpus)}" + ("  (**警告: 複数の CPU が混在**)" if len(cpus) > 1 else ""))
    e0 = envs[-1] if envs else {}
    L.append(f"- 論理 CPU 数: {e0.get('logical_cpus')}、メモリ: {_f(e0.get('ram_gb'), 1)} GB、OS: {e0.get('os')}")
    if e0.get("dmi_product"):
        L.append(f"- インスタンス: {e0.get('dmi_vendor')} {e0.get('dmi_product')}、CPU governor: {e0.get('cpu_governor')}")
    hosts = sorted({e.get("hostname") for e in envs})
    L.append(f"- 計測ホスト: {', '.join(h or '?' for h in hosts)}")
    vers = e0.get("versions", {})
    L.append(f"- ENOMOTO: {vers.get('enomoto')} ({e0.get('rustc')})、HiGHS {vers.get('highs')} (highspy)、"
             f"CLP {vers.get('clp')}、SoPlex {vers.get('soplex')}")
    if e0.get("env_overrides"):
        L.append(f"- 環境変数の上書き: {e0['env_overrides']}")
    thr = st.get("threads", 0)
    thr_note = (f"。スレッド数: ENOMOTO・HiGHS は {thr} (論理 CPU 数 {e0.get('logical_cpus')})、"
                "CLP・SoPlex は 1 (逐次のソルバー)") if thr else "。スレッド数は各ソルバーの既定値"
    L.append(f"- 設定: 各ソルバーの既定値 (時間制限とスレッド数のみ設定){thr_note}。"
             "ENOMOTO は実行不能と非有界を区別する設定")
    L.append(f"- 制限時間 {limit:.0f} 秒。各問題を {st['reps']} 回解いて中央値 "
             f"(1 回目が {st['single_run_above_s']:.0f} 秒以上、または解けなかったときは 1 回)。"
             "1 回の求解ごとに新しいプロセス、繰り返しの間でソルバーの順番を回す")
    if st.get("warmup_below_s"):
        L.append(f"- 1 回目が {st['warmup_below_s']:g} 秒未満の問題は、その 1 回をウォームアップとして捨てて測り直す")
    L.append("- 時間は求解のみ (MPS の読み込みとモデル構築は含まない)、単位は秒")
    L.append(f"- 解けた = 期待どおりの状態で、最適なら目的関数値が基準値と相対 {rtol:g} 以内。"
             "解けなかった問題は制限時間として平均・総時間に含める")
    L.append(f"- 幾何平均・シフト付き幾何平均 (shift {SHIFT:.0f} 秒)・総時間")
    if excluded:
        L.append(f"- 集計から除いた問題: {', '.join(sorted(excluded))}")
    renamed = sorted({f"{k.split('/', 1)[1]} ({SOLVER_LABEL[s]})" for k, runs_of in data["runs"].items()
                      if k in cells for s, runs in runs_of.items() if any(r.get("renamed_mps") for r in runs)})
    if renamed:
        L.append("- MPS の名前に空白を含むなどで元のファイルを読めなかったため、HiGHS で読み直して名前を"
                 f"付け直した MPS で解いたもの: {', '.join(renamed)}")
    L.append("")

    def agg_table(keys: list[str], title: str) -> None:
        keys = [k for k in keys if k in cells]
        if not keys:
            return
        both = [k for k in keys if all(s in cells[k] and cells[k][s].solved for s in solvers)]
        L.append(f"### {title} ({len(keys)} 問)\n")
        L.append("| ソルバー | 解けた数 | 幾何平均 | シフト付き幾何平均 | 総時間 | "
                 f"全ソルバーが解けた {len(both)} 問の幾何平均 |")
        L.append("|---|---:|---:|---:|---:|---:|")
        for s in solvers:
            ts = [cells[k][s].time if s in cells[k] else limit for k in keys]
            solved = sum(1 for k in keys if s in cells[k] and cells[k][s].solved)
            tb = [cells[k][s].time for k in both]
            L.append(f"| {SOLVER_LABEL[s]} | {solved}/{len(keys)} | {_f(gm(ts), 4)} | {_f(sgm(ts), 4)} | "
                     f"{_f(sum(ts), 2)} | {_f(gm(tb), 4) if tb else '-'} |")
        L.append("")

    L.append("## 集計\n")
    by_set = {s: [k for k in cells if k.split("/", 1)[0] == s] for s in SETS}
    for s in SETS:
        agg_table(by_set[s], SET_LABEL[s])
    agg_table([k for k in cells if k.split("/", 1)[0] in ("netlib", "kennington", "mittelmann")],
              "有限最適解をもつ問題の合計 (Netlib + Kennington + Mittelmann)")
    agg_table([k for k in cells if k.split("/", 1)[0] in ("infeas", "infeas_dual")],
              "実行不能・非有界の合計 (Netlib 実行不能 + その双対)")
    agg_table(list(cells), "全問題")

    # ---- ENOMOTO の段階ごとの時刻 (実行不能・非有界の問題)
    inf_keys = [k for k in cells if k.split("/", 1)[0] in ("infeas", "infeas_dual") and "enomoto" in cells[k]]
    if inf_keys:
        L.append("## ENOMOTO の段階ごとの判定時刻 (実行不能 29 問 + 双対 29 問)\n")
        L.append("時刻はいずれも求解の開始 (前処理の直前) からの秒。T_A = 段階 A の終わりで z¹ < 0 "
                 "(実行不能か非有界) を検出した時刻、T_B = 段階 B の後に実行不能/非有界を結論した時刻 "
                 "(前処理が結論した問題はその時刻)、T_total = Model.solve() 全体。\n")
        for group, title in (("infeas", SET_LABEL["infeas"]), ("infeas_dual", SET_LABEL["infeas_dual"]), (None, "合計")):
            ks = [k for k in inf_keys if group is None or k.split("/", 1)[0] == group]
            ok = [k for k in ks if cells[k]["enomoto"].solved and cells[k]["enomoto"].phase]
            ka = [k for k in ok if cells[k]["enomoto"].phase["t_a"] is not None]
            states: dict[str, int] = {}
            for k in ok:
                a = cells[k]["enomoto"].phase["a_state"]
                states[a] = states.get(a, 0) + 1
            L.append(f"### {title}: 正しく結論 {len(ok)}/{len(ks)} 問、段階 A で検出 {len(ka)} 問 "
                     f"(内訳: {', '.join(f'{a} {n}' for a, n in sorted(states.items()))})\n")
            L.append("| 量 | 問題数 | 幾何平均 | シフト付き幾何平均 | 総時間 |")
            L.append("|---|---:|---:|---:|---:|")
            rows = [
                ("T_A (段階 A で検出した問題)", [cells[k]["enomoto"].phase["t_a"] for k in ka]),
                ("T_B (同じ問題)", [cells[k]["enomoto"].phase["t_verdict"] for k in ka]),
                ("T_total (同じ問題)", [cells[k]["enomoto"].time for k in ka]),
                ("T_B (正しく結論した全問題)", [cells[k]["enomoto"].phase["t_verdict"] for k in ok
                                          if cells[k]["enomoto"].phase["t_verdict"] is not None]),
                ("T_total (正しく結論した全問題)", [cells[k]["enomoto"].time for k in ok]),
            ]
            for label, ts in rows:
                L.append(f"| {label} | {len(ts)} | {_f(gm(ts), 4)} | {_f(sgm(ts), 4)} | {_f(sum(ts), 3)} |")
            L.append("")
        L.append("| 問題 | 期待 | 段階 A | T_A | T_B | 結論の出所 | T_total | "
                 + " | ".join(SOLVER_LABEL[s] for s in solvers if s != "enomoto") + " |")
        L.append("|---|---|---|---:|---:|---|---:|" + "---:|" * (len(solvers) - 1))
        for k in inf_keys:
            c = cells[k]["enomoto"]
            ph = c.phase or {}
            prob = all_problems.get(k)
            others = [_cell_str(cells[k].get(s)) for s in solvers if s != "enomoto"]
            L.append(f"| {k.split('/', 1)[1]} | {prob.expected if prob else '?'} | {ph.get('a_state', c.status)} | "
                     f"{_f(ph.get('t_a'), 4)} | {_f(ph.get('t_verdict'), 4)} | {ph.get('verdict_source') or c.status} | "
                     f"{_cell_str(c)} | " + " | ".join(others) + " |")
        L.append("")

    # ---- 揺らぎの目安
    L.append("## 揺らぎの目安\n")
    L.append("3 回以上測れた問題について、(最大 − 最小) / 中央値 を求解時間の帯ごとに示す。\n")
    L.append("| ソルバー | 時間帯 | 問題数 | 中央値 | 90% 点 | 最大 |")
    L.append("|---|---|---:|---:|---:|---:|")
    buckets = [(0, 0.01), (0.01, 0.1), (0.1, 1), (1, 10), (10, 100), (100, math.inf)]
    for s in solvers:
        spreads: dict[tuple, list[float]] = {b: [] for b in buckets}
        for key, runs_of in data["runs"].items():
            ts = [r["time"] for r in runs_of.get(s, []) if r.get("status") in DEFINITIVE and r.get("time")]
            if key not in cells or len(ts) < 3:
                continue
            med = statistics.median(ts)
            for b in buckets:
                if b[0] <= med < b[1]:
                    spreads[b].append((max(ts) - min(ts)) / med)
        for b, xs in spreads.items():
            if not xs:
                continue
            xs.sort()
            p90 = xs[min(len(xs) - 1, int(math.ceil(0.9 * len(xs))) - 1)]
            label = f"{b[0]:g}–{b[1]:g} 秒" if b[1] != math.inf else f"{b[0]:g} 秒以上"
            L.append(f"| {SOLVER_LABEL[s]} | {label} | {len(xs)} | {statistics.median(xs) * 100:.1f}% | "
                     f"{p90 * 100:.1f}% | {xs[-1] * 100:.1f}% |")
    L.append("")
    single = [(k, s, cells[k][s]) for k in cells for s in solvers
              if s in cells[k] and cells[k][s].n_runs == 1 and cells[k][s].solved]
    if single:
        L.append(f"1 回だけ測った (時間のかかる) 解けた問題: {len(single)} 件 — "
                 + ", ".join(f"{k.split('/', 1)[1]} ({SOLVER_LABEL[s]} {_f(c.time)} 秒)" for k, s, c in single))
        L.append("")
    if data.get("variability"):
        L.append("### 追加計測 (--variability-probe)\n")
        L.append("| 問題 | ソルバー | 回数 | 最小 | 中央値 | 最大 | (最大−最小)/中央値 |")
        L.append("|---|---|---:|---:|---:|---:|---:|")
        for key, runs_of in data["variability"].items():
            for s in solvers:
                ts = [r["time"] for r in runs_of.get(s, []) if r.get("status") in DEFINITIVE and r.get("time")]
                if not ts:
                    continue
                med = statistics.median(ts)
                L.append(f"| {key} | {SOLVER_LABEL[s]} | {len(ts)} | {_f(min(ts))} | {_f(med)} | {_f(max(ts))} | "
                         f"{(max(ts) - min(ts)) / med * 100:.1f}% |")
        L.append("")

    # ---- 問題ごとの表
    L.append("## 問題ごとの求解時間 (中央値、秒)\n")
    L.append("`t` = 制限時間超過、`m` = メモリ不足、`f` = 期待と違う結論・目的関数値の不一致・異常終了 "
             "(括弧内はその状態)、`¹` = 1 回だけ測った。\n")
    for s_name in SETS:
        ks = by_set[s_name]
        if not ks:
            continue
        L.append(f"### {SET_LABEL[s_name]}\n")
        L.append("| 問題 | " + " | ".join(SOLVER_LABEL[s] for s in solvers) + " |")
        L.append("|---|" + "---:|" * len(solvers))
        for k in sorted(ks, key=lambda k: all_problems[k].size if k in all_problems else 0):
            L.append(f"| {k.split('/', 1)[1]} | " + " | ".join(_cell_str(cells[k].get(s)) for s in solvers) + " |")
        L.append("")

    text = "\n".join(L) + "\n"
    (out_dir / "summary.md").write_text(text, encoding="utf-8")

    with open(out_dir / "per_problem.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["set", "problem", "expected", "reference_obj", "solver", "n_runs", "status", "solved",
                    "median_time_s", "run_times_s", "obj", "enomoto_stage_a", "enomoto_t_a_s",
                    "enomoto_t_verdict_s", "enomoto_verdict_source"])
        for k, cs in cells.items():
            prob = all_problems.get(k)
            for s, c in cs.items():
                ph = c.phase or {}
                w.writerow([k.split("/", 1)[0], k.split("/", 1)[1], prob.expected if prob else None, refs.get(k), s,
                            c.n_runs, c.status, c.solved, c.time, " ".join(_f(t, 6) for t in c.run_times), c.obj,
                            ph.get("a_state"), ph.get("t_a"), ph.get("t_verdict"), ph.get("verdict_source")])
    return text


def _cell_str(c: Cell | None) -> str:
    if c is None:
        return "-"
    if c.solved:
        return _f(c.time) + ("" if c.n_runs > 1 else "¹")
    code = {"timeout": "t", "limit": "t", "memory": "m", "read_timeout": "t"}.get(c.status or "", "f")
    return code if code != "f" else f"f ({c.status})"


def list_problems_safe() -> list[Problem]:
    """集計用: データの無い集合は飛ばして問題の一覧を作る。"""
    out: list[Problem] = []
    for s in SETS:
        try:
            out.extend(list_problems([s]))
        except SystemExit:
            pass
    return out


def merge_inputs(paths: list[Path]) -> dict:
    merged: dict = {"settings": {}, "environments": [], "runs": {}, "variability": {}, "warmup": {}}
    for p in paths:
        d = json.loads(p.read_text())
        if merged["settings"] and d["settings"].get("time_limit_s") != merged["settings"].get("time_limit_s"):
            raise SystemExit(f"{p}: 制限時間が他の入力と違う")
        merged["settings"] = merged["settings"] or d["settings"]
        merged["environments"].extend(d.get("environments", []))
        for key in ("runs", "variability", "warmup"):
            for prob, by_solver in d.get(key, {}).items():
                tgt = merged[key].setdefault(prob, {})
                for s, runs in by_solver.items():
                    tgt.setdefault(s, []).extend(runs)
    return merged


# ================================================================ main

def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sets", nargs="*", default=SETS, choices=SETS)
    ap.add_argument("--solvers", nargs="*", default=SOLVERS, choices=SOLVERS)
    ap.add_argument("--only", nargs="*", help="問題名 (または 集合/問題名) で絞る")
    ap.add_argument("--exclude", nargs="*", help="計測・集計から除く問題名 (または 集合/問題名)")
    ap.add_argument("--time-limit", type=float, default=3600.0)
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--single-run-above", type=float, default=600.0,
                    help="1 回目がこの秒数以上かかったら繰り返さない")
    ap.add_argument("--warmup-below", type=float, default=1.0,
                    help="1 回目がこの秒数未満なら、それを捨てて (ウォームアップ) 測り直す (0 で無効)")
    ap.add_argument("--threads", type=int, default=-1,
                    help="HiGHS/ENOMOTO のスレッド数 (-1 = 論理 CPU 数 (既定)、0 = 各ソルバーの既定値)")
    ap.add_argument("--mem-gb", type=float, default=None,
                    help="1 プロセスのメモリ上限 GB (Linux、既定: 物理メモリの 90%%、0 で無制限)")
    ap.add_argument("--read-timeout", type=float, default=3.0 * 3600, help="MPS 読み込み・モデル構築の上限秒")
    ap.add_argument("--obj-rtol", type=float, default=1e-6)
    ap.add_argument("--clp-bin", type=Path, default=REPO / ".paper_solvers" / "bin" / "clp_driver")
    ap.add_argument("--soplex-bin", type=Path, default=REPO / ".paper_solvers" / "bin" / "soplex_driver")
    ap.add_argument("--out-dir", type=Path, default=REPO / "benchmarks" / "paper")
    ap.add_argument("--shard", help="i/n: 問題を n 台に分けたときの i 台目 (0 始まり)")
    ap.add_argument("--variability-probe", nargs="*", help="揺らぎの目安のために追加で繰り返し解く問題")
    ap.add_argument("--probe-reps", type=int, default=3)
    ap.add_argument("--retry-statuses", nargs="*",
                    help="この状態で終わった回がある問題 × ソルバーを測り直す (例: read_error crash)")
    ap.add_argument("--report-only", action="store_true", help="計測せず results.json から表を作り直す")
    ap.add_argument("--inputs", nargs="*", type=Path, help="--report-only で結合する results.json (複数台の結果)")
    ap.add_argument("--worker", nargs="+", help=argparse.SUPPRESS)
    args = ap.parse_args()

    if args.worker:
        kind, mps = args.worker[0], args.worker[1]
        if kind == "enomoto":
            worker_enomoto(mps)
        else:
            worker_highs(mps, args.time_limit, args.threads)
        return

    args.out_dir.mkdir(parents=True, exist_ok=True)
    if args.report_only:
        data = merge_inputs(args.inputs) if args.inputs else _load(args.out_dir / "results.json")
        print(build_report(data, args.out_dir, args.exclude))
        return

    if args.mem_gb is None:
        args.mem_gb = 0.0
        if hasattr(os, "sysconf"):
            try:
                args.mem_gb = 0.9 * os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES") / 2**30
            except (ValueError, OSError):
                pass
    if args.threads < 0:
        args.threads = os.cpu_count() or 1  # 並列計算の効果も含めて測るため、既定は最大
    for s, b in (("clp", args.clp_bin), ("soplex", args.soplex_bin)):
        if s in args.solvers and not b.exists():
            raise SystemExit(f"{b} が無い: bash scripts/paper_bench/setup_solvers.sh を先に実行すること")
    run_benchmark(args)
    print(build_report(_load(args.out_dir / "results.json"), args.out_dir, args.exclude))


if __name__ == "__main__":
    main()
