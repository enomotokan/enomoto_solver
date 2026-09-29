"""Mittelmann LPopt benchmark (https://plato.asu.edu/ftp/lpopt.html): this
crate vs HiGHS.

Downloads the publicly available LPopt instances from
`plato.asu.edu/ftp/lptestset/` into `<repo>/.mittelmann_cache/raw/` (once;
the cache is `.gitignore`d, like `.netlib_cache`), then solves each problem
with both solvers, each in its own subprocess:

- HiGHS: default options plus `time_limit`, timed around `Highs.run()`.
- this crate: the MPS file is parsed by HiGHS and the model built with
  `benchmark_highs._build_our_model` (the same path as the Netlib
  benchmark, minus an O(m^2) vector copy; see `_build_our_model` here).
  Only `model.solve()` is timed. This crate has no time-limit
  option, so the parent kills the worker once `--timeout` seconds have
  passed since the worker reported that model building finished.

Each worker runs under an address-space limit (`--mem-gb`), so a problem too
large for the machine is recorded as "m" instead of taking the container
down.

Results are written after every problem to `--out` (JSON), and problems
already there are skipped, so an interrupted run can be resumed by running
the same command again. `--table` rebuilds the Markdown/CSV tables from the
JSON without solving anything.

Problems that are part of LPopt but not hosted on plato.asu.edu (rail02,
shs1023, stp3d from miplib2010.zib.de; stat96v2, degme from sztaki.hu) and
the 16 undisclosed instances are not included.

Linux only (uses `resource` / `preexec_fn` for the per-worker memory limit).

Usage: python scripts/run_mittelmann_benchmark.py [--timeout 600] [--only NAME ...] [--table]
"""
from __future__ import annotations

import argparse
import bz2
import csv
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

BASE_URL = "https://plato.asu.edu/ftp/lptestset/"

# (name in the LPopt table, path under lptestset/, rows, cols, nnz from lpopt.html).
# Paths without an `.mps` suffix are in Netlib's compressed format and need `emps`.
PROBLEMS: list[tuple[str, str, int, int, int]] = [
    ("L1_sixm250obs", "L1_sixm250obs.bz2", 986069, 428032, 4280320),
    ("Linf_520c", "Linf_520c.bz2", 93326, 69004, 566193),
    ("a2864", "a2864.mps.bz2", 22117, 200787, 20078717),
    ("bdry2", "bdry2.bz2", 376500, 250998, 1500003),
    ("cont1", "misc/cont1.bz2", 160793, 250998, 399991),
    ("cont11", "misc/cont11.bz2", 160793, 80396, 439989),
    ("datt256", "datt256_lp.mps.bz2", 11077, 262144, 1503732),
    ("dlr1", "dlr1.mps.bz2", 1735470, 9121907, 18365107),
    ("ex10", "ex10.mps.bz2", 69609, 17680, 1179680),
    ("fhnw-binschedule1", "fhnw-binschedule1.mps.bz2", 772872, 1141653, 8611326),
    ("fome13", "fome/fome13.bz2", 48569, 97840, 334984),
    ("graph40-40", "graph40-40.mps.bz2", 360900, 102600, 1260900),
    ("irish-electricity", "irish-electricity.mps.bz2", 104260, 61728, 538809),
    ("neos", "misc/neos.bz2", 479120, 36786, 1084461),
    ("neos3", "misc/neos3.bz2", 512209, 6624, 1542816),
    ("neos-3025225", "neos-3025225.mps.bz2", 91572, 69846, 9357951),
    ("neos-5052403-cygnet", "neos-5052403-cygnet.mps.bz2", 38269, 32868, 4898304),
    ("neos-5251015", "neos-5251015.mps.bz2", 486531, 136971, 1955388),
    ("ns1687037", "misc/ns1687037.bz2", 50622, 43749, 1406739),
    ("ns1688926", "misc/ns1688926.bz2", 32768, 16587, 1712128),
    ("nug08-3rd", "nug/nug08-3rd.bz2", 19728, 20448, 139008),
    ("pds-100", "pds/pds-100.bz2", 156244, 505360, 1390539),
    ("physiciansched3-3", "physiciansched3-3.mps.bz2", 266228, 79555, 1062480),
    ("qap15", "qap15.mps.bz2", 6331, 22275, 110700),
    ("rail4284", "rail/rail4284.bz2", 4284, 1092610, 12372358),
    ("rmine15", "rmine15.mps.bz2", 358395, 42438, 879732),
    ("s82", "s82.mps.bz2", 87878, 1690631, 7022608),
    ("s100", "s100.mps.bz2", 14734, 364417, 2127672),
    ("s250r10", "s250r10.mps.bz2", 10963, 273142, 1572104),
    ("savsched1", "savsched1.mps.bz2", 295990, 328575, 1846351),
    ("scpm1", "scpm1.mps.bz2", 5000, 500000, 6250000),
    ("square41", "square41.mps.bz2", 40161, 62234, 13628623),
    ("stormG2_1000", "misc/stormG2_1000.bz2", 528186, 1259121, 4228817),
    ("supportcase10", "supportcase10.mps.bz2", 165685, 14770, 551152),
    ("tpl-tub-ws1617", "tpl-tub-ws1617.mps.bz2", 1154615, 747691, 4720567),
    ("woodlands09", "woodlands09.mps.bz2", 194599, 382147, 2646003),
    ("Dual2_5000", "Dual2_5000.mps.bz2", 30000600, 33050602, 93001800),
    ("Primal2_1000", "Primal2_1000.mps.bz2", 1299380, 2559380, 5498140),
    ("thk_48", "thk_48.mps.bz2", 6366377, 8609262, 27802878),
    ("thk_63", "thk_63.mps.bz2", 5694387, 7701112, 21592414),
    ("L1_sixm1000obs", "L1_sixm1000obs.bz2", 3082940, 1426256, 14262560),
    ("L2CTA3D", "L2CTA3D.mps.bz2", 210000, 10000000, 30000000),
    ("dlr2", "dlr2.mps.bz2", 7132926, 38868107, 78091589),
    ("set-cover-model", "set-cover-model.mps.bz2", 10000, 1102008, 20442268),
]
NOT_AVAILABLE = ["rail02", "shs1023", "stp3d", "stat96v2", "degme"]

SHIFT = 10.0  # seconds, as in Mittelmann's "shifted geometric mean"


# ---------------------------------------------------------------- download

def _download(url: str, dest: Path) -> None:
    tmp = dest.with_suffix(dest.suffix + ".part")
    for attempt in range(5):
        try:
            with urllib.request.urlopen(url, timeout=120) as r, open(tmp, "wb") as f:
                shutil.copyfileobj(r, f, 1 << 20)
            tmp.rename(dest)
            return
        except Exception as e:  # noqa: BLE001
            if attempt == 4:
                raise
            print(f"  retry {url}: {e}", flush=True)
            time.sleep(2 ** (attempt + 1))


def ensure_raw(cache: Path, name: str, rel: str) -> Path:
    raw = cache / "raw" / rel.replace("/", "__")
    raw.parent.mkdir(parents=True, exist_ok=True)
    if not raw.exists():
        print(f"  downloading {rel}", flush=True)
        _download(BASE_URL + rel, raw)
    return raw


def ensure_emps(cache: Path) -> Path:
    from netlib_fetch import ensure_emps as _ensure

    return _ensure(cache, network=True)


def materialize_mps(cache: Path, rel: str, raw: Path, workdir: Path) -> Path:
    """Decompresses `raw` into `workdir` (deleted by the caller after the
    problem is done, so only one uncompressed MPS is on disk at a time)."""
    out = workdir / "problem.mps"
    if rel.endswith(".mps.bz2"):
        with bz2.open(raw, "rb") as src, open(out, "wb") as dst:
            shutil.copyfileobj(src, dst, 1 << 20)
        return out
    packed = workdir / "problem.emps"
    with bz2.open(raw, "rb") as src, open(packed, "wb") as dst:
        shutil.copyfileobj(src, dst, 1 << 20)
    with open(out, "wb") as dst:
        subprocess.run([str(ensure_emps(cache)), str(packed)], stdout=dst, check=True)
    packed.unlink()
    return out


# ---------------------------------------------------------------- workers

def _worker_highs(mps: str, time_limit: float) -> None:
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", time_limit)
    t0 = time.perf_counter()
    status = h.readModel(mps)
    read_time = time.perf_counter() - t0
    if "kOk" not in str(status) and "kWarning" not in str(status):
        print(json.dumps({"status": "read_error", "detail": str(status)}))
        return
    t0 = time.perf_counter()
    h.run()
    t = time.perf_counter() - t0
    st = str(h.getModelStatus()).split(".")[-1]
    info = h.getInfo()
    print(json.dumps({
        "time": t,
        "read_time": read_time,
        "status": st,
        "obj": h.getObjectiveValue() if st == "kOptimal" else None,
        "iters": int(info.simplex_iteration_count),
        "ipm_iters": int(info.ipm_iteration_count),
    }))


def _worker_ours(mps: str) -> None:
    import highspy

    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    status = h.readModel(mps)
    if "kOk" not in str(status) and "kWarning" not in str(status):
        print(json.dumps({"status": "read_error", "detail": str(status)}))
        return
    t0 = time.perf_counter()
    model = _build_our_model(h.getLp())
    build_time = time.perf_counter() - t0
    del h
    print(json.dumps({"built": True, "build_time": build_time}), flush=True)
    t0 = time.perf_counter()
    out = model.solve(root_solver=None)
    t = time.perf_counter() - t0
    print(json.dumps({"time": t, "build_time": build_time, "status": out["status"], "obj": out.get("objective")}))


def _build_our_model(lp):
    """Same model as `enomoto_solver.benchmark_highs._build_our_model`, but
    copies each HiGHS vector out once: indexing `lp.row_lower_[i]` etc. on
    the pybind object copies the whole vector per access, which is O(m^2)
    and takes minutes already at ~10^5 rows."""
    from enomoto_solver import _core

    m = _core.PyModel()
    n = lp.num_col_
    col_lower, col_upper = list(lp.col_lower_), list(lp.col_upper_)
    integrality = list(lp.integrality_)
    for j in range(n):
        lb, ub = float(col_lower[j]), float(col_upper[j])
        if lb > ub:
            lb, ub = ub, lb
        is_int = len(integrality) > j and int(integrality[j]) != 0
        m.add_variable("integer" if is_int else "continuous", lb, ub)

    obj_coeffs = [(j, float(c)) for j, c in enumerate(list(lp.col_cost_)) if c != 0.0]
    sense = "maximize" if "kMaximize" in str(lp.sense_) else "minimize"
    m.set_objective(obj_coeffs, float(lp.offset_), sense)

    n_rows = lp.num_row_
    rows: list[list[tuple[int, float]]] = [[] for _ in range(n_rows)]
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
        rows[i] = None  # type: ignore[call-overload]  # free as we go
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


def _limit_memory(mem_gb: float):
    def _set() -> None:
        import resource  # Linux 専用なので使うときだけ読む (問題一覧などは他の OS からも使える)

        lim = int(mem_gb * (1 << 30))
        resource.setrlimit(resource.RLIMIT_AS, (lim, lim))
    return _set


def _classify_crash(rc: int, stderr: str) -> str:
    if "MemoryError" in stderr or "memory allocation" in stderr or "bad_alloc" in stderr or rc in (-6, -9):
        return "m"
    return "f"


def run_highs(mps: Path, timeout: float, mem_gb: float) -> dict:
    cmd = [sys.executable, __file__, "--worker-highs", str(mps), "--timeout", str(timeout)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout * 2 + 600,
                           preexec_fn=_limit_memory(mem_gb))
    except subprocess.TimeoutExpired:
        return {"code": "t", "status": "killed"}
    lines = [l for l in p.stdout.splitlines() if l.startswith("{")]
    if p.returncode != 0 or not lines:
        return {"code": _classify_crash(p.returncode, p.stderr), "status": f"crash rc={p.returncode}",
                "stderr": p.stderr[-500:]}
    r = json.loads(lines[-1])
    if r.get("status") == "kOptimal":
        r["code"] = "ok"
    elif r.get("status") == "kTimeLimit":
        r["code"] = "t"
    elif r.get("status") == "kMemoryLimit":
        r["code"] = "m"
    else:
        r["code"] = "f"
    return r


def run_ours(mps: Path, timeout: float, mem_gb: float, build_timeout: float) -> dict:
    cmd = [sys.executable, __file__, "--worker-ours", str(mps)]
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                         preexec_fn=_limit_memory(mem_gb))
    lines: list[str] = []
    built = threading.Event()

    def _reader() -> None:
        for line in p.stdout:  # type: ignore[union-attr]
            lines.append(line.strip())
            if '"built"' in line:
                built.set()

    err_chunks: list[str] = []
    th = threading.Thread(target=_reader, daemon=True)
    th_err = threading.Thread(target=lambda: err_chunks.append(p.stderr.read()), daemon=True)  # type: ignore[union-attr]
    th.start()
    th_err.start()

    t_start = time.time()
    while not built.is_set() and p.poll() is None and time.time() - t_start < build_timeout:
        time.sleep(0.2)
    if p.poll() is None and not built.is_set():
        p.kill()
        p.wait()
        return {"code": "t", "status": "model build timed out"}
    try:
        p.wait(timeout=timeout + 5)  # +5s: margin for the worker's own solve() entry/exit
    except subprocess.TimeoutExpired:
        p.kill()
        p.wait()
        th.join(5)
        build = next((json.loads(l) for l in lines if '"built"' in l), {})
        return {"code": "t", "status": "killed", "build_time": build.get("build_time")}
    th.join(5)
    th_err.join(5)
    stderr = "".join(err_chunks)
    results = [json.loads(l) for l in lines if l.startswith("{") and '"built"' not in l]
    if p.returncode != 0 or not results:
        return {"code": _classify_crash(p.returncode, stderr), "status": f"crash rc={p.returncode}",
                "stderr": stderr[-500:]}
    r = results[-1]
    if r.get("time", 0) > timeout:
        r["code"] = "t"
    elif r.get("status") == "optimal":
        r["code"] = "ok"
    else:
        r["code"] = "f"
    return r


# ---------------------------------------------------------------- tables

def _obj_match(a, b) -> bool | None:
    if a is None or b is None:
        return None
    return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))


def _eff_time(r: dict, timeout: float) -> float:
    return r["time"] if r.get("code") == "ok" else timeout


def sgm(ts: list[float]) -> float:
    return math.exp(sum(math.log(t + SHIFT) for t in ts) / len(ts)) - SHIFT


def gm(ts: list[float]) -> float:
    return math.exp(sum(math.log(max(t, 1e-6)) for t in ts) / len(ts))


def _fmt_t(r: dict) -> str:
    if not r:
        return "-"
    if r.get("code") == "ok":
        t = r["time"]
        return f"{t:.2f}" if t < 100 else f"{t:.0f}"
    return r.get("code", "?")


def write_tables(data: dict, md_path: Path, csv_path: Path) -> str:
    timeout = data["config"]["timeout_s"]
    probs = data["problems"]
    order = [p[0] for p in PROBLEMS if p[0] in probs]
    size = {p[0]: p[2:] for p in PROBLEMS}

    rows = []
    for name in order:
        r = probs[name]
        h, o = r.get("highs", {}), r.get("ours", {})
        ratio = None
        if h.get("code") == "ok" and o.get("code") == "ok":
            ratio = o["time"] / h["time"]
        rows.append((name, h, o, ratio, _obj_match(h.get("obj"), o.get("obj"))))

    with open(csv_path, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["name", "rows", "cols", "nnz", "highs_code", "highs_time", "highs_status", "highs_obj",
                    "ours_code", "ours_time", "ours_status", "ours_obj", "ratio_ours_over_highs", "obj_match"])
        for name, h, o, ratio, om in rows:
            w.writerow([name, *size[name], h.get("code"), h.get("time"), h.get("status"), h.get("obj"),
                        o.get("code"), o.get("time"), o.get("status"), o.get("obj"), ratio, om])

    n = len(rows)
    h_eff = [_eff_time(h, timeout) for _, h, _, _, _ in rows]
    o_eff = [_eff_time(o, timeout) for _, _, o, _, _ in rows]
    h_solved = sum(1 for _, h, _, _, _ in rows if h.get("code") == "ok")
    o_solved = sum(1 for _, _, o, _, _ in rows if o.get("code") == "ok")
    both = [(h["time"], o["time"]) for _, h, o, _, _ in rows if h.get("code") == "ok" and o.get("code") == "ok"]

    L = []
    L.append("# Mittelmann LPopt ベンチマーク: enomoto_solver vs HiGHS\n")
    L.append(f"- 実行日時: {data['config']['generated_at']}")
    L.append(f"- マシン: {data['config']['machine']}")
    L.append(f"- HiGHS {data['config']['highs_version']} (既定オプション、`time_limit`={timeout:.0f}s)")
    L.append(f"- enomoto_solver: `model.solve()` (既定の拡張双対単体法)。{timeout:.0f}s で打ち切り")
    L.append("- 時間は求解のみ(MPS読み込み・モデル構築は含まない)。単位は秒")
    mem = data["config"].get("mem_note") or f"{data['config']['mem_gb']:.0f}GB"
    L.append(f"- `t` = 制限時間超過、`m` = メモリ不足 (上限 {mem})、`f` = 失敗・最適以外の終了")
    L.append(f"- 対象: 公開 49 問のうち plato.asu.edu から取得できた {len(PROBLEMS)} 問"
             f"(取得不可: {', '.join(NOT_AVAILABLE)}、および非公開の 16 問)\n")
    L.append("## 集計\n")
    L.append("| 指標 | HiGHS | enomoto | 比 (enomoto/HiGHS) |")
    L.append("|---|---:|---:|---:|")
    L.append(f"| 解けた問題数 (/{n}) | {h_solved} | {o_solved} | |")
    sh, so = sgm(h_eff), sgm(o_eff)
    L.append(f"| Shifted geomean (shift {SHIFT:.0f}s、未解決={timeout:.0f}s 扱い、全{n}問) | {sh:.2f} | {so:.2f} | **{so / sh:.2f}x** |")
    if both:
        gh, go = gm([b[0] for b in both]), gm([b[1] for b in both])
        L.append(f"| 幾何平均 (両方解けた {len(both)} 問のみ) | {gh:.3f} | {go:.3f} | **{go / gh:.2f}x** |")
    else:
        L.append("| 幾何平均 (両方解けた問題のみ) | - | - | 該当なし |")
    L.append("")
    L.append("## 問題ごとの求解時間\n")
    L.append("| 問題 | 行 | 列 | 非零 | HiGHS [s] | enomoto [s] | 比 | 目的関数一致 |")
    L.append("|---|---:|---:|---:|---:|---:|---:|:---:|")
    for name, h, o, ratio, om in rows:
        m_, n_, nz = size[name]
        rs = f"{ratio:.2f}x" if ratio is not None else "-"
        oms = {True: "✓", False: "✗", None: "-"}[om]
        L.append(f"| {name} | {m_:,} | {n_:,} | {nz:,} | {_fmt_t(h)} | {_fmt_t(o)} | {rs} | {oms} |")
    text = "\n".join(L) + "\n"
    md_path.write_text(text)
    return text


# ---------------------------------------------------------------- main

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".mittelmann_cache")
    ap.add_argument("--out", type=Path, default=REPO_ROOT / "benchmarks" / "mittelmann_results.json")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--build-timeout", type=float, default=3600.0)
    ap.add_argument("--mem-gb", type=float, default=12.0)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--download-only", action="store_true")
    ap.add_argument("--table", action="store_true", help="only rebuild the tables from --out")
    ap.add_argument("--worker-highs")
    ap.add_argument("--worker-ours")
    args = ap.parse_args()

    if args.worker_highs:
        _worker_highs(args.worker_highs, args.timeout)
        return
    if args.worker_ours:
        _worker_ours(args.worker_ours)
        return

    md_path = args.out.with_suffix(".md")
    csv_path = args.out.with_suffix(".csv")
    if args.table:
        print(write_tables(json.loads(args.out.read_text()), md_path, csv_path))
        return

    import highspy

    args.cache_dir.mkdir(parents=True, exist_ok=True)
    data = json.loads(args.out.read_text()) if args.out.exists() else {"problems": {}}
    data["config"] = {
        "timeout_s": args.timeout,
        "mem_gb": args.mem_gb,
        "highs_version": highspy.Highs().version(),
        "machine": f"{os.cpu_count()} CPU, {os.sysconf('SC_PAGE_SIZE') * os.sysconf('SC_PHYS_PAGES') / 2**30:.0f}GB RAM (Linux)",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC"),
    }

    todo = [p for p in PROBLEMS if not args.only or p[0] in args.only]
    for name, rel, *_ in todo:
        ensure_raw(args.cache_dir, name, rel)
    if args.download_only:
        return

    todo.sort(key=lambda p: p[4])  # smallest first, so results arrive early
    for name, rel, *_ in todo:
        if name in data["problems"] and not args.only:
            continue
        t0 = time.time()
        raw = ensure_raw(args.cache_dir, name, rel)
        workdir = Path(tempfile.mkdtemp(dir=args.cache_dir))
        try:
            mps = materialize_mps(args.cache_dir, rel, raw, workdir)
            h = run_highs(mps, args.timeout, args.mem_gb)
            o = run_ours(mps, args.timeout, args.mem_gb, args.build_timeout)
        finally:
            shutil.rmtree(workdir, ignore_errors=True)
        data["problems"][name] = {"highs": h, "ours": o}
        args.out.write_text(json.dumps(data, indent=1, sort_keys=True))
        write_tables(data, md_path, csv_path)
        print(f"{name:22s} highs={_fmt_t(h):>8s} ({h.get('status')})  ours={_fmt_t(o):>8s} ({o.get('status')})"
              f"  [{time.time() - t0:.0f}s]", flush=True)

    print(write_tables(data, md_path, csv_path))


if __name__ == "__main__":
    main()
