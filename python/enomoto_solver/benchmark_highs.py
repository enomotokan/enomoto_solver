"""実行時間の比較: このクレートの単体法エンジン vs. HiGHS (`highspy` 経由)。
Netlib の LP ベンチマーク集合のうち、列数が `--max-vars` (既定 3000) 以下の問題で比較する。

MPS ファイルの無限大境界も含め、各問題は HiGHS が読み込んだままの形でこのクレートに
渡す (このスクリプトでは境界の置き換えをしない)。目的関数値も健全性確認として比較する。

使い方:
    python -m enomoto_solver.benchmark_highs [--max-vars 3000] [--timeout 60]

Netlib の LP データはリポジトリに含まれず (docs/netlib-data.md 参照)、このスクリプトは
netlib.org にアクセスしない。環境ごとに一度 `python scripts/setup_netlib_data.py` を実行して
`--cache-dir` (既定: `<repo>/.netlib_cache`、gitignore 済み) にデータを用意すること
(Netlib 独自の圧縮 MPS 形式を Netlib の `emps.c` で展開するので、`gcc` などの C コンパイラが
`PATH` に必要)。このスクリプトはそのキャッシュを読むだけ。
"""


from __future__ import annotations

import argparse
import csv
import json
import math
import subprocess
import sys
import time
from pathlib import Path

import highspy

from . import _core


class NetlibCacheError(RuntimeError):
    """必要な Netlib データがキャッシュにない。環境ごとに一度 (リポジトリのチェックアウトから)
    `scripts/setup_netlib_data.py` を実行すること。"""


def _cached_problem_list(cache_dir: Path) -> list[str]:
    """キャッシュの `problems.txt` から問題名の一覧を読む。なければ NetlibCacheError。"""
    list_path = cache_dir / "problems.txt"
    if not list_path.exists():
        raise NetlibCacheError(f"{list_path} not found — run `python scripts/setup_netlib_data.py` once to fetch it")
    return [line.strip() for line in list_path.read_text().splitlines() if line.strip()]


def _cached_mps(name: str, cache_dir: Path) -> Path | None:
    """問題 `name` の展開済み MPS ファイルのパス。存在しないか空なら None。"""
    mps_path = cache_dir / "mps" / f"{name}.mps"
    return mps_path if mps_path.exists() and mps_path.stat().st_size > 0 else None


def _load_lp(mps_path: Path):
    """`mps_path` を HiGHS で読み込み `(highs_instance, lp)` を返す。`highs_instance` は
    HiGHS 側の計測にそのまま使うので、両ソルバーは同じ読み込み結果を解く。"""
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    status = h.readModel(str(mps_path))
    if "kOk" not in str(status):
        raise RuntimeError(f"readModel failed: {status}")
    return h, h.getLp()


def _build_our_model(lp) -> tuple[_core.PyModel, int, int]:
    """HiGHS が読み込んだ LP データから、このクレートの `PyModel` を直接組み立てる
    (項ごとの Python オブジェクトの負担を避けるため `Variable`/`Constraint` の DSL は使わない)。
    境界は `+/-inf` も含め HiGHS が読んだとおりに渡す。範囲制約 (両側有限で幅のあるもの) は
    `<=` と `>=` の 2 行に分ける。`(model, 制約数, 非零数)` を返す。"""
    m = _core.PyModel()
    n = lp.num_col_
    # highspy は属性アクセスのたびにベクトル全体を複製するので、各属性は 1 回だけ読む
    # (要素ごとに `lp.col_lower_[j]` とすると O(n^2) になる)。
    col_lower = [float(v) for v in lp.col_lower_]
    col_upper = [float(v) for v in lp.col_upper_]
    integrality = [int(v) for v in lp.integrality_]
    row_lower = [float(v) for v in lp.row_lower_]
    row_upper = [float(v) for v in lp.row_upper_]

    for j in range(n):
        lb, ub = col_lower[j], col_upper[j]
        if lb > ub:
            lb, ub = ub, lb
        is_int = len(integrality) > j and integrality[j] != 0
        m.add_variable("integer" if is_int else "continuous", lb, ub)

    obj_coeffs = [(j, float(c)) for j, c in enumerate(lp.col_cost_) if c != 0.0]
    sense = "maximize" if "kMaximize" in str(lp.sense_) else "minimize"
    m.set_objective(obj_coeffs, float(lp.offset_), sense)

    n_rows = lp.num_row_
    # 行ごとの非零 (列, 値) の一覧 (列優先・行優先どちらの格納形式からも作る)
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

    n_constraints = 0
    for i in range(n_rows):
        lo, hi = row_lower[i], row_upper[i]
        terms = rows[i]
        if math.isinf(lo) and math.isinf(hi):
            continue  # 両側無限 (自由行) は制約にならない
        if not math.isinf(lo) and abs(hi - lo) < 1e-12:  # 上下限がほぼ等しければ等式
            m.add_constraint(terms, "==", float(lo))
            n_constraints += 1
            continue
        if not math.isinf(hi):
            m.add_constraint(terms, "<=", float(hi))
            n_constraints += 1
        if not math.isinf(lo):
            m.add_constraint(terms, ">=", float(lo))
            n_constraints += 1

    nnz = sum(len(r) for r in rows)
    return m, n_constraints, nnz


def _run_one(name: str, mps_path: Path, solve_timeout: float) -> dict:
    """問題 1 つを HiGHS とこのクレートの両方で解き、時間・状態・目的関数値を dict で返す。
    途中の例外は送出せず `*error` キーに記録する。(`solve_timeout` は未使用。
    時間制限は呼び出し元のサブプロセスで課す。)"""
    result: dict = {"name": name}
    try:
        h, lp = _load_lp(mps_path)
    except Exception as e:  # noqa: BLE001 - reported, not raised
        result["error"] = f"read failed: {e}"
        return result

    result["n_vars"] = lp.num_col_
    result["n_rows"] = lp.num_row_

    try:
        t0 = time.perf_counter()
        h.run()
        highs_time = time.perf_counter() - t0
        status = str(h.getModelStatus())
        result["highs_time"] = highs_time
        result["highs_status"] = status
        result["highs_obj"] = h.getObjectiveValue() if "kOptimal" in status else None
    except Exception as e:  # noqa: BLE001
        result["highs_error"] = str(e)

    try:
        model, n_constraints, nnz = _build_our_model(lp)
        result["n_constraints"] = n_constraints
        result["nnz"] = nnz
        t0 = time.perf_counter()
        out = model.solve(root_solver=None)
        ours_time = time.perf_counter() - t0
        result["ours_time"] = ours_time
        result["ours_status"] = out["status"]
        result["ours_obj"] = out["objective"]
    except Exception as e:  # noqa: BLE001
        result["ours_error"] = str(e)

    return result


def _run_worker(name: str, cache_dir: Path, timeout: float) -> None:
    """`--worker` の入口: このプロセスで問題を 1 つだけ解き、結果を JSON 1 行で標準出力に
    出す。`main()` からサブプロセスとして呼ばれる。"""
    mps_path = _cached_mps(name, cache_dir)
    if mps_path is None:
        print(json.dumps({"name": name, "error": "could not decompress"}))
        return
    print(json.dumps(_run_one(name, mps_path, timeout)))


def main() -> None:
    """コマンドライン入口。各問題をサブプロセスで解いて結果を表示し、CSV に書き出し、
    両ソルバーが最適に解けた問題の合計時間と比を表示する。"""
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--max-vars", type=int, default=3000, help="skip problems with more columns than this")
    parser.add_argument("--timeout", type=float, default=60.0, help="per-problem wall-clock budget, seconds — enforced (SIGTERM/kill) via a per-problem subprocess")
    # `__file__` ではなくカレントディレクトリ基準 (wheel からインストールすると
    # `__file__` は site-packages 下になりリポジトリに辿れないため)。
    # `scripts/setup_netlib_data.py` の既定値も `<cwd>/.netlib_cache` なので、
    # 両方をリポジトリのルートから実行すれば一致する。
    parser.add_argument("--cache-dir", type=Path, default=Path.cwd() / ".netlib_cache")
    parser.add_argument("--out", type=Path, default=Path.cwd() / "netlib_benchmark_results.csv")
    parser.add_argument("--only", nargs="*", help="run only these problem names (default: all, size-filtered)")
    parser.add_argument("--worker", metavar="NAME", help=argparse.SUPPRESS)  # 内部用: 問題 1 つだけを解くサブプロセスモード
    args = parser.parse_args()

    args.cache_dir.mkdir(parents=True, exist_ok=True)

    if args.worker:
        _run_worker(args.worker, args.cache_dir, args.timeout)
        return

    try:
        names = args.only if args.only else _cached_problem_list(args.cache_dir)
    except NetlibCacheError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)

    # 各問題の結果 dict
    rows = []
    for name in names:
        mps_path = _cached_mps(name, args.cache_dir)
        if mps_path is None:
            print(f"{name:12s} SKIP (not cached — run `python scripts/setup_netlib_data.py` to fetch it)")
            continue

        # 大きすぎる問題をサブプロセスに渡さないための安価な事前チェック
        # (`readModel` は最大級の Netlib 問題でも速い)。
        h, lp = _load_lp(mps_path)
        n_vars = lp.num_col_
        del h, lp
        if n_vars > args.max_vars:
            print(f"{name:12s} SKIP (n_vars={n_vars} > {args.max_vars})")
            continue

        # 実際の求解は問題ごとのサブプロセスで行う。Rust 側の panic が PyO3 境界を越えると
        # インタプリタごと落ちるので、1 問の失敗でバッチ全体が止まらないようにするため。
        # また `--timeout` を実際に効かせる (遅い/固まった求解を kill する) ためでもある。
        try:
            proc = subprocess.run(
                [sys.executable, "-m", "enomoto_solver.benchmark_highs", "--worker", name, "--cache-dir", str(args.cache_dir)],
                capture_output=True,
                text=True,
                timeout=args.timeout,
            )
        except subprocess.TimeoutExpired:
            result = {"name": name, "error": f"timed out after {args.timeout}s"}
            rows.append(result)
            print(f"{name:12s} ERROR: {result['error']}")
            continue

        if proc.returncode != 0 or not proc.stdout.strip():
            stderr_tail = proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "(no stderr)"
            result = {"name": name, "error": f"worker crashed (exit {proc.returncode}): {stderr_tail}"}
            rows.append(result)
            print(f"{name:12s} ERROR: {result['error']}")
            continue

        result = json.loads(proc.stdout.strip().splitlines()[-1])
        rows.append(result)

        if "error" in result:
            print(f"{name:12s} ERROR: {result['error']}")
            continue
        # ot/ht: このクレート/HiGHS の時間、os_/hs: それぞれの状態
        ot = result.get("ours_time")
        ht = result.get("highs_time")
        os_ = result.get("ours_status", "?")
        hs = result.get("highs_status", "?")
        ratio = f"{ot / ht:6.2f}x" if isinstance(ot, float) and isinstance(ht, float) and ht > 0 else "   n/a"
        print(
            f"{name:12s} n={result.get('n_vars', '?'):>6} m={result.get('n_rows', '?'):>6}  "
            f"ours={ot if ot is not None else float('nan'):8.4f}s [{os_:10s}]  "
            f"highs={ht if ht is not None else float('nan'):8.4f}s [{hs:20s}]  ratio(ours/highs)={ratio}"
        )

    fieldnames = sorted({k for r in rows for k in r.keys()})
    with open(args.out, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in rows:
            writer.writerow(r)
    print(f"\nwrote {len(rows)} rows to {args.out}")

    # 両ソルバーが最適に解けた問題だけで合計時間を比べる
    solved = [r for r in rows if r.get("ours_status") == "optimal" and r.get("highs_status") and "kOptimal" in r["highs_status"]]
    if solved:
        total_ours = sum(r["ours_time"] for r in solved)
        total_highs = sum(r["highs_time"] for r in solved)
        print(f"\n{len(solved)} problems solved to optimality by both:")
        print(f"  total ours time:  {total_ours:.4f}s")
        print(f"  total highs time: {total_highs:.4f}s")
        print(f"  ratio (ours/highs): {total_ours / total_highs:.2f}x")


if __name__ == "__main__":
    main()
