"""Mittelmann 先生のベンチマークに出す前の検算 (誤答がないこと、2026-11-03)。

58 問 (analysis/cbcimpl_20261029/probs.py の PROBS) + Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) の重複を除いた
203 問の並び (58 問の順のあとに Mittelmann の残りを名前順) の K 番目から N おきを、今の既定 (環境変数なし = canddet + p) で
4 スレッド・60 秒で解き、次を確かめて check_sK.jsonl に 1 問 1 行で足す (書いてある問題は飛ばすので途中から再開できる)。

- 返した解 x (元の列の順) を、元の MPS の問題 (前処理の前。highspy で読んだ行・列・境界・整数性) に代入して
  行の違反・境界の違反・整数性の違反と目的値 (c x + 定数) を計算し直す。違反は 1e-6 * max(1, |右辺|) (境界は |境界|) を超えたら NG、
  整数性は |x - round(x)| > 1e-6 で NG。参考に 1e-9 を超えた数も残す。報告の目的値と計算し直した値の差は相対 1e-6 で NG。
- 状態と目的値・限界値を MIPLIB 2017 の solu (scripts/miplib2017-v31.solu、=opt=/=best=/=inf=) と比べる (相対 1e-6 = 1e-6 * max(1, |参照|))。
  - optimal: =opt= と食い違えば opt_mismatch、=best= より悪ければ opt_worse_than_best (=best= より良いのは better_than_best として記録のみ)、
    =inf= なら opt_on_infeasible。
  - infeasible: =opt=/=best= の問題なら wrong_infeasible。
  - 時間切れ等の暫定解: =opt=/=best= より良ければ better_than_known (解の違反を疑う)。
  - 限界値 (best_bound): 既知の最良値より (最小化なら) 大きければ bound_invalid。
  - HiGHS の結果 (analysis/highs_mitt_20261103/highs_s*.jsonl・analysis/highs_vs_20261101/highs.jsonl) も参考に並べ、
    両方 optimal で目的値が食い違えば highs_mismatch、片方だけ実行不能なら highs_inf_mismatch。
- 各行の "issues" が空なら合格。crash・killed・解なしの optimal も issues に入れる。

使い方 (リポジトリの根で、ソルバーは同時に 1 つ):
    pip install -q highspy==1.15.1 maturin numpy
    maturin build --release -o /tmp/whl && pip install -q --force-reinstall --no-deps /tmp/whl/*.whl
    python analysis/mittelmann_ready_20261103/check_sol.py K 6      # K = 0..5 (分担)。結果は同じ場所の check_sK.jsonl
    python analysis/mittelmann_ready_20261103/check_sol.py --only P1 P2 --out x.jsonl   # 問題を指名して解き直す (再現の確かめ)
    python analysis/mittelmann_ready_20261103/check_sol.py --check-only K 6 # 解き直さずに一覧 (並び) を見るだけ
環境変数 CHECK_TL (既定 60)・CHECK_TH (既定 4) で時間の上限・スレッド数を変えられる (ベンチマークの条件は既定のまま)。
解き手はサブプロセス (このファイルの --worker) で、x は一時ファイル (.npy) で受け取る。
"""
import json, math, os, subprocess, sys, tempfile, time
from pathlib import Path

D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
TL = float(os.environ.get("CHECK_TL", "60"))
TH = int(os.environ.get("CHECK_TH", "4"))
TOL = 1e-6


def all_problems() -> list:
    from probs import PROBS  # noqa: E402
    mitt = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
    return list(dict.fromkeys(list(PROBS) + mitt))


def load_solu() -> dict:
    ref = {}
    for l in (WT / "scripts/miplib2017-v31.solu").read_text().splitlines():
        t = l.split()
        if len(t) >= 2:
            ref[t[1]] = (t[0].strip("="), float(t[2]) if len(t) >= 3 else None)
    return ref


def load_highs() -> dict:
    """HiGHS の結果 (問題 -> [result, ...])。"""
    out = {}
    files = sorted((WT / "analysis/highs_mitt_20261103").glob("highs_s*.jsonl")) + [WT / "analysis/highs_vs_20261101/highs.jsonl"]
    for f in files:
        if not f.exists():
            continue
        for l in f.read_text().splitlines():
            if l.strip():
                r = json.loads(l)
                out.setdefault(r["problem"], []).append(r["result"])
    return out


def worker(mps: str, xout: str) -> None:
    import numpy as np
    from enomoto_solver import benchmark_highs as bh
    _, lp = bh._load_lp(Path(mps))
    m, _, _ = bh._build_our_model(lp)
    t = time.perf_counter()
    r = m.solve(None, False, TL, None, None, TH)
    t = time.perf_counter() - t
    x = r.get("x")
    if x is not None:
        np.save(xout, np.asarray(x, dtype=float))
    print(json.dumps({"status": r["status"], "objective": r["objective"], "bound": r.get("best_bound"), "nodes": r.get("nodes"),
                      "time": t, "has_x": x is not None, "n_x": None if x is None else len(x)}))


def check_x(lp, x) -> dict:
    """x を元の問題 (highspy で読んだ lp) に代入して違反と目的値を計算する。"""
    import numpy as np
    n, m = lp.num_col_, lp.num_row_
    out = {}
    if len(x) != n:
        return {"len_mismatch": [len(x), n]}
    if not np.all(np.isfinite(x)):
        out["nonfinite"] = int(np.sum(~np.isfinite(x)))
        x = np.nan_to_num(x)
    lo = np.asarray(lp.col_lower_, dtype=float); hi = np.asarray(lp.col_upper_, dtype=float)
    integ = np.asarray([int(v) for v in lp.integrality_], dtype=int) if len(lp.integrality_) else np.zeros(n, dtype=int)
    if np.any(integ >= 2):
        out["semi_cols"] = int(np.sum(integ >= 2))  # 半連続 (このソルバーは整数として渡している) は境界と整数性を別に見る
    # 境界
    lv = np.where(np.isfinite(lo), lo - x, 0.0); hv = np.where(np.isfinite(hi), x - hi, 0.0)
    semi0 = (integ >= 2) & (np.abs(x) <= TOL)  # 半連続の 0 は下限の違反でない
    lv[semi0] = 0.0
    bv = np.maximum(np.maximum(lv, hv), 0.0)
    bscale = np.maximum(1.0, np.maximum(np.where(np.isfinite(lo), np.abs(lo), 0), np.where(np.isfinite(hi), np.abs(hi), 0)))
    out["bound_viol_max"] = float(bv.max()) if n else 0.0
    out["bound_viol_n"] = int(np.sum(bv > TOL * bscale))
    out["bound_viol_n_1e9"] = int(np.sum(bv > 1e-9 * bscale))
    # 整数性
    isint = (integ == 1) | (integ == 3)
    fr = np.abs(x - np.round(x)) * isint
    out["int_viol_max"] = float(fr.max()) if n else 0.0
    out["int_viol_n"] = int(np.sum(fr > TOL))
    out["int_viol_n_1e9"] = int(np.sum(fr > 1e-9))
    # 行
    am = lp.a_matrix_
    start = np.asarray(am.start_, dtype=np.int64); index = np.asarray(am.index_, dtype=np.int64); value = np.asarray(am.value_, dtype=float)
    cnt = np.diff(start)
    if "kColwise" in str(am.format_):
        cols = np.repeat(np.arange(n), cnt); rows = index[: start[-1]]
    else:
        rows = np.repeat(np.arange(m), cnt); cols = index[: start[-1]]
    value = value[: start[-1]]
    prod = value * x[cols]
    act = np.bincount(rows, weights=prod, minlength=m)
    absact = np.bincount(rows, weights=np.abs(prod), minlength=m)
    rlo = np.asarray(lp.row_lower_, dtype=float); rhi = np.asarray(lp.row_upper_, dtype=float)
    rv = np.maximum(np.maximum(np.where(np.isfinite(rlo), rlo - act, 0.0), np.where(np.isfinite(rhi), act - rhi, 0.0)), 0.0)
    rscale = np.maximum(1.0, np.maximum(np.where(np.isfinite(rlo), np.abs(rlo), 0), np.where(np.isfinite(rhi), np.abs(rhi), 0)))
    out["row_viol_max"] = float(rv.max()) if m else 0.0
    out["row_viol_rel_max"] = float((rv / rscale).max()) if m else 0.0
    # 項の大きさ (sum |a_ij x_j|) で割った違反も参考に (打ち消し合いの大きい行)
    out["row_viol_relact_max"] = float((rv / np.maximum(rscale, absact)).max()) if m else 0.0
    out["row_viol_n"] = int(np.sum(rv > TOL * rscale))
    out["row_viol_n_1e9"] = int(np.sum(rv > 1e-9 * rscale))
    if out["row_viol_n"]:
        worst = np.argsort(-(rv / rscale))[:3]
        out["row_viol_worst"] = [[int(i), float(rv[i]), float(act[i]), float(rlo[i]), float(rhi[i])] for i in worst]
    c = np.asarray(lp.col_cost_, dtype=float)
    out["obj_recomputed"] = float(c @ x + float(lp.offset_))
    return out


def better(a, b, maximize: bool, tol: float) -> bool:
    """a が b より (目的の向きで) tol を超えて良い。"""
    return (a > b + tol) if maximize else (a < b - tol)


def check_one(p: str, solu: dict, highs: dict, mps: Path) -> dict:
    import numpy as np
    from enomoto_solver import benchmark_highs as bh
    with tempfile.TemporaryDirectory() as td:
        xp = os.path.join(td, "x.npy")
        env = dict(os.environ, **({"ENOMOTO_MIP_THREADS": str(TH)} if TH > 1 else {}))
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, __file__, "--worker", str(mps), xp], cwd=WT, env=env, capture_output=True, text=True,
                               timeout=TL * 1.5 + 120)
            lines = [l for l in r.stdout.splitlines() if l.startswith("{")]
            res = json.loads(lines[-1]) if lines else {"status": f"crash rc={r.returncode}", "stderr": r.stderr[-1500:]}
            n_warn = r.stderr.count("MIP: WARNING")
            if n_warn:
                res["mip_warnings"] = n_warn
                res["mip_warning_text"] = list(dict.fromkeys(l.strip() for l in r.stderr.splitlines() if "MIP: WARNING" in l))[:5]
        except subprocess.TimeoutExpired:
            res = {"status": "killed"}
        wall = time.time() - t0
        x = np.load(xp) if os.path.exists(xp) else None
    rec = {"problem": p, "result": res, "wall": wall}
    issues = []
    st = res.get("status")
    if st not in ("optimal", "infeasible", "time_limit", "node_limit", "not_solved", "unbounded", "infeasible_or_unbounded"):
        issues.append(f"bad_status:{st}")
    _, lp = bh._load_lp(mps)
    maximize = "kMaximize" in str(lp.sense_)
    rec["maximize"] = maximize
    if x is not None:
        chk = check_x(lp, x)
        rec["check"] = chk
        if "len_mismatch" in chk:
            issues.append("x_len_mismatch")
        else:
            if chk.get("nonfinite"): issues.append("x_nonfinite")
            if chk["bound_viol_n"]: issues.append("bound_viol")
            if chk["int_viol_n"]: issues.append("int_viol")
            if chk["row_viol_n"]: issues.append("row_viol")
            obj = res.get("objective")
            if obj is None or abs(obj - chk["obj_recomputed"]) > TOL * max(1.0, abs(chk["obj_recomputed"])):
                issues.append("obj_report_mismatch")
    elif st == "optimal":
        issues.append("optimal_without_x")
    kind, val = solu.get(p, ("unknown", None))
    rec["ref"] = [kind, val]
    obj = rec.get("check", {}).get("obj_recomputed", res.get("objective"))
    if kind in ("opt", "best") and val is not None:
        tol = TOL * max(1.0, abs(val))
        if st == "optimal" and obj is not None:
            if kind == "opt" and abs(obj - val) > tol:
                issues.append("opt_mismatch")
            elif kind == "best" and better(val, obj, maximize, tol):
                issues.append("opt_worse_than_best")
            elif kind == "best" and better(obj, val, maximize, tol):
                rec["note"] = "better_than_best"
        elif obj is not None and better(obj, val, maximize, tol):
            issues.append("better_than_known")
        if st in ("infeasible", "infeasible_or_unbounded"):
            issues.append("wrong_infeasible")
        if st == "unbounded":
            issues.append("wrong_unbounded")
        b = res.get("bound")
        if b is not None and math.isfinite(b) and better(b, val, maximize, tol):
            issues.append("bound_invalid")
    elif kind == "inf":
        if st == "optimal" or x is not None:
            issues.append("solution_on_infeasible")
    # HiGHS との突き合わせ (参考)
    hs = highs.get(p, [])
    rec["highs"] = [[h.get("status"), h.get("objective"), h.get("bound")] for h in hs]
    for h in hs:
        if h.get("status") == "Optimal" and st == "optimal" and obj is not None and h.get("objective") is not None:
            if abs(h["objective"] - obj) > TOL * max(1.0, abs(h["objective"])):
                issues.append("highs_mismatch")
        if (h.get("status") == "Infeasible") != (st == "infeasible") and (h.get("status") == "Optimal" or st == "optimal"):
            issues.append("highs_inf_mismatch")
    rec["issues"] = sorted(set(issues))
    return rec


def main() -> None:
    a = sys.argv[1:]
    if a and a[0] == "--worker":
        return worker(a[1], a[2])
    import run_miplib_benchmark as rb  # noqa: E402
    probs = all_problems()
    if a and a[0] == "--only":
        i = a.index("--out")
        mine, out = a[1:i], Path(a[i + 1])
    else:
        check_only = a and a[0] == "--check-only"
        if check_only:
            a = a[1:]
        k, n = int(a[0]), int(a[1])
        mine, out = probs[k::n], D / f"check_s{k}.jsonl"
        if check_only:
            print(len(probs), len(mine), mine)
            return
    solu, highs = load_solu(), load_highs()
    done = {json.loads(l)["problem"] for l in out.read_text().splitlines() if l.strip()} if out.exists() and "--only" not in sys.argv else set()
    for p in mine:
        if p in done:
            continue
        for attempt in range(5):
            try:
                mps = rb.fetch(p, WT / ".miplib_cache"); break
            except Exception as e:  # noqa: BLE001
                err = e; time.sleep(30)
        else:
            with open(out, "a") as f: f.write(json.dumps({"problem": p, "result": {"status": "fetch_error", "error": str(err)[:200]}, "issues": ["fetch_error"]}) + "\n")
            continue
        try:
            rec = check_one(p, solu, highs, mps)
        except Exception as e:  # noqa: BLE001
            rec = {"problem": p, "issues": ["checker_error"], "error": repr(e)[:300]}
        rec["at"] = time.strftime("%H:%M:%S")
        with open(out, "a") as f: f.write(json.dumps(rec) + "\n")
        r = rec.get("result", {})
        print(f"{p:32s} {str(r.get('status')):>12s} obj={r.get('objective')} ref={rec.get('ref')} issues={rec['issues']}", flush=True)


if __name__ == "__main__":
    main()
