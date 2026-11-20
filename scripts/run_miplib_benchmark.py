"""MIPLIB 2017 の小〜中規模の問題で、このクレートの分枝限定法と HiGHS を比べる。

問題は https://miplib.zib.de/WebData/instances/<name>.mps.gz から `<repo>/.miplib_cache/` に取得する
(一度だけ。キャッシュは .gitignore 済み)。各問題を両ソルバーでそれぞれ別のサブプロセスで解く:

- HiGHS: 既定の設定 + `time_limit` + `mip_rel_gap = 0` (このクレートの既定の相対ギャップ 0 にそろえる。2026-10-20 から。
  それより前の結果は HiGHS の既定 1e-4)。`Highs.run()` の時間を測る。
- このクレート: MPS はこのクレートの読み込み (`benchmark_highs._read_our_model` → `PyModel.read_mps`) で読む
  (2026-11-20 から。それより前は HiGHS で読んで `benchmark_highs._build_our_model` で組み立てていた)。
  `model.solve(time_limit=...)` の時間だけを測る。親プロセスは時間上限 + 余裕で打ち切る。

結果は問題ごとに `--out` (JSON) に書き足すので、中断しても同じコマンドで再開できる。
`--table` は JSON から Markdown の表だけを作り直す。

使い方: python scripts/run_miplib_benchmark.py [--time-limit 60] [--threads N] [--only NAME ...] [--table]
"""

from __future__ import annotations

import argparse
import gzip
import json
import math
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
BASE_URL = "https://miplib.zib.de/WebData/instances/"

# MIPLIB 2017 の collection から選んだ、HiGHS が数秒〜数十秒で解く程度の問題 (正しさと速度の確認用)。
PROBLEMS = [
    "gen-ip002", "gen-ip054", "pk1", "gr4x6", "markshare_4_0", "mas74", "mas76", "neos5", "p0201",
    "misc07", "flugpl", "dcmulti", "gt2", "khb05250", "qnet1", "rout", "10teams", "air03", "blend2",
    "fiber", "noswot", "qiu", "neos-911970", "30n20b8", "binkar10_1", "neos-860300", "eil33-2", "nw04",
    "assign1-5-8", "beavma", "blp-ar98", "ran14x18-disj-8", "h80x6320d", "supportcase26", "timtab1",
    "bppc4-08", "cvs16r128-89", "mik-250-20-75-4", "neos-1456979", "graph20-20-1rand",
]



def _read_our_model_compat(bh, path):
    """`benchmark_highs._read_our_model` (このクレートの MPS 読み込み) でモデルを作る。古いビルドを wheel で入れた
    環境では `benchmark_highs` 自体が古く `_read_our_model` がないので、従来の HiGHS 経由の組み立てに戻る
    (台本だけ新しくなった計測が落ちないように)。戻り値は `(model, 制約数, 非零数, 列の名前)`。"""
    if hasattr(bh, "_read_our_model"):
        return bh._read_our_model(path)
    _, lp = bh._load_lp(path)
    names = list(lp.col_names_) if len(lp.col_names_) == lp.num_col_ else [f"C{j}" for j in range(lp.num_col_)]
    model, n_constraints, nnz = bh._build_our_model(lp)
    return model, n_constraints, nnz, names

def fetch(name: str, cache: Path) -> Path:
    """`name` の MPS を取得して展開し、そのパスを返す。"""
    cache.mkdir(parents=True, exist_ok=True)
    mps = cache / f"{name}.mps"
    if mps.exists() and mps.stat().st_size > 0:
        return mps
    gz = cache / f"{name}.mps.gz"
    with urllib.request.urlopen(BASE_URL + f"{name}.mps.gz", timeout=120) as r, open(gz, "wb") as f:
        shutil.copyfileobj(r, f)
    with gzip.open(gz, "rb") as src, open(mps, "wb") as dst:
        shutil.copyfileobj(src, dst)
    gz.unlink()
    return mps


def worker_highs(mps: str, time_limit: float, threads: int) -> None:
    import highspy

    h = highspy.Highs()
    # `ENOMOTO_BENCH_HIGHS_LOG=1`: HiGHS のログを出す (調べるとき用。結果の JSON は最後の行なのでそのまま読める)
    h.setOptionValue("output_flag", bool(os.environ.get("ENOMOTO_BENCH_HIGHS_LOG")))
    h.setOptionValue("time_limit", time_limit)
    h.setOptionValue("threads", threads)
    # このクレートの既定 (相対ギャップ 0) にそろえる。HiGHS の既定 1e-4 のままだと、根の LP と最適値の差が 1e-5 しかない
    # uccase12 などで HiGHS だけが 1 ノードで「最適」と打ち切り、比べられない
    h.setOptionValue("mip_rel_gap", 0.0)
    # `ENOMOTO_BENCH_HIGHS_PARALLEL=on`: HiGHS の並列の分枝限定法を使う (1.15.1 は既定の parallel=choose では threads を
    # 与えても木の探索が 1 本。on で "Parallel search on" になり、neos-911970 で CPU 時間 / 経過時間 1.04 -> 4.40)
    if os.environ.get("ENOMOTO_BENCH_HIGHS_PARALLEL"):
        h.setOptionValue("parallel", os.environ["ENOMOTO_BENCH_HIGHS_PARALLEL"])
    # `ENOMOTO_BENCH_HIGHS_OPT=名前=値,名前=値`: 調べるとき用に HiGHS のオプションを足す (例: mip_detect_symmetry=false)。
    # 値は true/false・整数・小数・文字列の順に読む
    for kv in filter(None, os.environ.get("ENOMOTO_BENCH_HIGHS_OPT", "").split(",")):
        k, v = kv.split("=", 1)
        val = {"true": True, "false": False}.get(v.lower(), v)
        if isinstance(val, str):
            for conv in (int, float):
                try:
                    val = conv(v)
                    break
                except ValueError:
                    pass
        h.setOptionValue(k, val)
    h.readModel(mps)
    t = time.perf_counter()
    h.run()
    t = time.perf_counter() - t
    info = h.getInfo()
    print(json.dumps({
        "status": h.modelStatusToString(h.getModelStatus()),
        "objective": info.objective_function_value,
        "bound": info.mip_dual_bound,
        "nodes": info.mip_node_count,
        "time": t,
    }))


# SCIP の状態を worker_ours の名前にそろえる (HiGHS は "Optimal" など別の名前のまま。summ 側で両方を読む)
SCIP_STATUS = {"optimal": "optimal", "infeasible": "infeasible", "unbounded": "unbounded", "inforunbd": "infeasible_or_unbounded",
               "timelimit": "time_limit", "memlimit": "memory_limit", "nodelimit": "node_limit", "gaplimit": "gap_limit",
               "userinterrupt": "interrupted"}


def worker_scip(mps: str, time_limit: float, threads: int) -> None:
    """SCIP (pyscipopt の wheel に同梱) で解く。SCIP は既定で逐次 (1 スレッド) なので threads は使わない (記録だけ)。
    時間の上限 limits/time、相対ギャップ limits/gap = 0 (HiGHS の mip_rel_gap = 0・このクレートの既定にそろえる。SCIP の既定も 0)。
    メモリの上限は付けない (limits/memory は既定の無限大のまま)。
    `ENOMOTO_BENCH_SCIP_SEED=N`: randomization/randomseedshift = N。`ENOMOTO_BENCH_SCIP_LOG=1`: SCIP のログを出す。"""
    import pyscipopt

    m = pyscipopt.Model()
    if not os.environ.get("ENOMOTO_BENCH_SCIP_LOG"):
        m.hideOutput()
    m.setParam("limits/time", time_limit)
    m.setParam("limits/gap", 0.0)
    seed = int(os.environ.get("ENOMOTO_BENCH_SCIP_SEED", "0"))
    if seed:
        m.setParam("randomization/randomseedshift", seed)
    m.readProblem(mps)
    t = time.perf_counter()
    m.optimize()
    t = time.perf_counter() - t
    st = m.getStatus()
    has = m.getNSols() > 0
    print(json.dumps({
        "status": SCIP_STATUS.get(st, st),
        "objective": m.getObjVal() if has else None,
        "bound": m.getDualbound(),
        "nodes": m.getNTotalNodes(),
        "time": t,
        "scip_time": m.getSolvingTime(),
        "scip_version": f"{m.getMajorVersion()}.{m.getMinorVersion()}.{m.getTechVersion()}",
        "pyscipopt_version": pyscipopt.__version__,
    }))


def worker_ours(mps: str, time_limit: float, threads: int) -> None:
    from enomoto_solver import benchmark_highs as bh

    m, _, _, _ = _read_our_model_compat(bh, Path(mps))
    t = time.perf_counter()
    r = m.solve(None, False, time_limit, None, None, threads)
    t = time.perf_counter() - t
    print(json.dumps({
        "status": r["status"],
        "objective": r["objective"],
        "bound": r["best_bound"],
        "nodes": r["nodes"],
        "time": t,
        "root_bound": r.get("root_bound"),
        "first_solution_time": r.get("first_solution_time"),
        "first_solution_objective": r.get("first_solution_objective"),
    }))


def primal_gap(obj, ref) -> float:
    """主ギャップ (SCIP/MIPLIB の定義): 解なし・符号が逆なら 1、それ以外は |obj - ref| / max(|obj|, |ref|)。"""
    if obj is None or ref is None:
        return 1.0
    if abs(obj - ref) <= 1e-9 * max(1.0, abs(ref)):
        return 0.0
    if obj * ref < 0:
        return 1.0
    return abs(obj - ref) / max(abs(obj), abs(ref))


def run_worker(kind: str, mps: Path, time_limit: float, threads: int) -> dict:
    cmd = [sys.executable, __file__, f"--worker-{kind}", str(mps), "--time-limit", str(time_limit), "--threads", str(threads)]
    # threads > 1 のとき、このクレートは木の探索 (threads) に加えて根のヒューリスティクス・サブ MIP も並列に
    # 走らせる (ENOMOTO_MIP_THREADS)
    env = dict(os.environ, ENOMOTO_MIP_THREADS=str(threads)) if threads > 1 else None
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=time_limit * 1.5 + 60, env=env)
    except subprocess.TimeoutExpired:
        return {"status": "killed", "objective": None, "bound": None, "nodes": None, "time": None}
    # ソルバーの警告 (部分木を失った疑いなど) は stderr にしか出ないので、件数を結果に残す
    n_warn = p.stderr.count("MIP: WARNING")
    for line in reversed(p.stdout.splitlines()):
        if line.startswith("{"):
            r = json.loads(line)
            if n_warn:
                r["mip_warnings"] = n_warn
                # 本文 (重複を除いて最初の 5 種類) も残す。件数だけでは何の警告か分からない
                texts = list(dict.fromkeys(l.strip() for l in p.stderr.splitlines() if "MIP: WARNING" in l))
                r["mip_warning_text"] = texts[:5]
            return r
    return {"status": f"crash rc={p.returncode}", "objective": None, "bound": None, "nodes": None, "time": None,
            "stderr": p.stderr[-2000:]}


def solved(r: dict) -> bool:
    return r.get("status") in ("Optimal", "optimal")


def make_table(results: dict, time_limit: float, threads: int) -> str:
    names = [n for n in PROBLEMS if n in results] + [n for n in results if n not in PROBLEMS]
    rows = []
    sg = {"highs": [], "ours": []}
    n_solved = {"highs": 0, "ours": 0}
    wrong = []
    pgap = {"highs": 0.0, "ours": 0.0}
    for n in names:
        h, o = results[n]["highs"], results[n]["ours"]
        for k, r in (("highs", h), ("ours", o)):
            t = r["time"] if solved(r) and r["time"] is not None else time_limit
            sg[k].append(t)
            n_solved[k] += solved(r)
        match = ""
        if solved(h) and solved(o):
            ok = abs(h["objective"] - o["objective"]) <= 1e-4 * max(1.0, abs(h["objective"])) + 1e-6
            match = "✓" if ok else "✗"
            if not ok:
                wrong.append(n)
        elif solved(h) != solved(o):
            match = "-"
        fmt = lambda r: f"{r['time']:.2f}" if solved(r) else ("t" if "time" in str(r["status"]).lower() or r["status"] == "killed" else str(r["status"]))
        fobj = lambda r: f"{r['objective']:.6g}" if r.get("objective") is not None else "-"
        # 主ギャップの参照値: 両者の目的値のうち良い方 (最小化・最大化は分からないので、解けた側があればその値)
        refv = h["objective"] if solved(h) else (o["objective"] if solved(o) else None)
        g = {k: primal_gap(r.get("objective"), refv) if refv is not None else None for k, r in (("highs", h), ("ours", o))}
        for k in g:
            pgap[k] += g[k] or 0.0
        fg = lambda v: "-" if v is None else f"{v:.4f}"
        frb = lambda v: "-" if v is None else f"{v:.6g}"
        ffs = lambda v: "-" if v is None else f"{v:.2f}"
        rows.append(f"| {n} | {fmt(h)} | {fmt(o)} | {fobj(h)} | {fobj(o)} | {h.get('nodes')} | {o.get('nodes')} | {match} "
                    f"| {fg(g['ours'])} | {frb(o.get('root_bound'))} | {ffs(o.get('first_solution_time'))} |")

    def shifted_geomean(ts, shift=10.0):
        return math.exp(sum(math.log(t + shift) for t in ts) / len(ts)) - shift if ts else float("nan")

    out = [
        "# MIPLIB 2017 (小〜中規模): enomoto_solver vs HiGHS",
        "",
        f"- 実行日時: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
        f"- 時間上限 {time_limit:g} 秒、スレッド数 {threads} (HiGHS は threads、enomoto は木の探索 threads と "
        f"ENOMOTO_MIP_THREADS)、相対ギャップは両方 0 (HiGHS は mip_rel_gap = 0)",
        "- 時間は求解のみ (MPS 読み込み・モデル構築を除く)。`t` = 時間上限",
        "",
        "| 指標 | HiGHS | enomoto |",
        "|---|---:|---:|",
        f"| 解けた問題数 (/{len(names)}) | {n_solved['highs']} | {n_solved['ours']} |",
        f"| Shifted geomean [s] (shift 10、未解決は上限扱い) | {shifted_geomean(sg['highs']):.2f} | {shifted_geomean(sg['ours']):.2f} |",
        f"| 目的値の不一致 | | {len(wrong)} {wrong if wrong else ''} |",
        f"| 主ギャップの和 (参照値は解けた側の目的値) | {pgap['highs']:.4f} | {pgap['ours']:.4f} |",
        "",
        "主ギャップ = |目的値 - 参照値| / max(|目的値|, |参照値|) (解なしは 1)。根の下界は enomoto の根の切除平面ループ後の LP 値"
        " (再スタートがあれば最後の根)、最初の解は enomoto の最初の暫定解の時刻。",
        "",
        "| 問題 | HiGHS [s] | enomoto [s] | HiGHS 目的値 | enomoto 目的値 | HiGHS ノード | enomoto ノード | 一致 | enomoto 主ギャップ | enomoto 根の下界 | enomoto 最初の解 [s] |",
        "|---|---:|---:|---:|---:|---:|---:|:---:|---:|---:|---:|",
    ] + rows
    return "\n".join(out) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--time-limit", type=float, default=60.0)
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--out", type=Path, default=REPO_ROOT / "benchmarks" / "miplib_results.json")
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".miplib_cache")
    ap.add_argument("--threads", type=int, default=1, help="両ソルバーに与えるスレッド数")
    ap.add_argument("--table", action="store_true")
    ap.add_argument("--rerun-ours", action="store_true", help="HiGHS の結果は再利用し、このクレートだけ解き直す")
    ap.add_argument("--rerun-highs", action="store_true", help="このクレートの結果は再利用し、HiGHS だけ解き直す")
    ap.add_argument("--worker-highs")
    ap.add_argument("--worker-ours")
    ap.add_argument("--worker-scip")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")  # 表の ✓ を cp932 のコンソールでも出せるように
    if a.worker_highs:
        return worker_highs(a.worker_highs, a.time_limit, a.threads)
    if a.worker_ours:
        return worker_ours(a.worker_ours, a.time_limit, a.threads)
    if a.worker_scip:
        return worker_scip(a.worker_scip, a.time_limit, a.threads)
    results = json.loads(a.out.read_text(encoding="utf-8")) if a.out.exists() else {}
    if not a.table:
        names = a.only or PROBLEMS
        for n in names:
            if n in results and not (a.rerun_ours or a.rerun_highs):
                continue
            mps = fetch(n, a.cache_dir)
            h = results.get(n, {}).get("highs") if a.rerun_ours else None
            if h is None:
                h = run_worker("highs", mps, a.time_limit, a.threads)
            o = results.get(n, {}).get("ours") if a.rerun_highs else None
            if o is None:
                o = run_worker("ours", mps, a.time_limit, a.threads)
            results[n] = {"highs": h, "ours": o}
            print(f"{n:24s} highs={h['status']:>10s} {h['time'] if h['time'] is not None else '-':>8} obj={h['objective']}  "
                  f"ours={o['status']:>10s} {o['time'] if o['time'] is not None else '-':>8} obj={o['objective']}"
                  + (f" MIP_WARNING={o['mip_warnings']}" if o.get('mip_warnings') else ""), flush=True)
            a.out.parent.mkdir(parents=True, exist_ok=True)
            a.out.write_text(json.dumps(results, indent=1), encoding="utf-8")
    md = make_table(results, a.time_limit, a.threads)
    a.out.with_suffix(".md").write_text(md, encoding="utf-8")
    print(md)


if __name__ == "__main__":
    main()
