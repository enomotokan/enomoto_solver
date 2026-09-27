"""特異・悪条件基底への耐性の回帰試験 (作業 #5、analysis/singular_basis_20260927_114500.md の M10)。

Netlib 93 問を数値的に厳しい 9 通りの設定 (LU の閾値を下げる、updateVerify を切る、FT 更新の下限を緩める、
再分解を減らす など) で解き、HiGHS の目的関数値と相対 1e-6 で照合する。`--cases` を付けると単体法側の
試験問題 2 問 (irish-electricity を旧前処理で、pilot87 を閾値 1e-3 で) も解き、元問題の制約違反も出す。

    python scripts/singular_stress.py [--settings base,s3] [--jobs 3] [--cases] [--mitt-dir /home/user/mitt]

HiGHS の参照値は `<cache>/highs_ref.json` に保存して再利用する。1 問 300 s で打ち切る。
"""

import argparse
import glob
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

# 9 通りのストレス設定 (/home/user/sp_sing/netlib_stress.py の s1〜s9 のうち s7 (`ENOMOTO_TINY`、既定オフの危険な設定) を除く)。
SETTINGS = {
    "base": {},
    "s1": {"ENOMOTO_DISABLE_UPDATE_VERIFY": "1"},
    "s2": {"ENOMOTO_T_FT_MIN_PIVOT": "1e-13", "ENOMOTO_T_STUCK_ROW_MIN_PIVOT": "0"},
    "s3": {"ENOMOTO_PIVOT_THRESHOLD": "1e-3"},
    "s4": {"ENOMOTO_SYNTH_CLOCK_FACTOR": "1000", "ENOMOTO_T_FT_MAX_UPDATES_FACTOR": "100"},
    "s5": {"ENOMOTO_DISABLE_UPDATE_VERIFY": "1", "ENOMOTO_T_FT_MIN_PIVOT": "1e-13", "ENOMOTO_T_STUCK_ROW_MIN_PIVOT": "0", "ENOMOTO_SYNTH_CLOCK_FACTOR": "1000"},
    "s6": {"ENOMOTO_PIVOT_THRESHOLD": "1e-3", "ENOMOTO_PIVOT_ESCALATION_STEP": "1"},
    "s8": {"ENOMOTO_PIVOT_THRESHOLD": "1e-3", "ENOMOTO_DISABLE_UPDATE_VERIFY": "1"},
    "s9": {"ENOMOTO_T_FT_MIN_PIVOT": "1e-13", "ENOMOTO_T_STUCK_ROW_MIN_PIVOT": "0", "ENOMOTO_DISABLE_UPDATE_VERIFY": "1", "ENOMOTO_PIVOT_THRESHOLD": "1e-3"},
}

# 1 問を解いて状態・目的関数値・元問題の最大違反を出す子プロセス。
WORKER = r"""
import sys, json
import numpy as np
sys.path.insert(0, sys.argv[2])
import highspy
from run_mittelmann_benchmark import _build_our_model
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp()
model = _build_our_model(lp)
out = model.solve(root_solver=None)
res = {"status": out["status"], "obj": out.get("objective")}
x = out.get("x")
if x is not None and out["status"] == "optimal":
    x = np.asarray(x, float)
    am = lp.a_matrix_
    st, idx, val = np.array(am.start_), np.array(am.index_), np.array(am.value_, float)
    cols = np.repeat(np.arange(lp.num_col_), np.diff(st))
    ax = np.bincount(idx, weights=val * x[cols], minlength=lp.num_row_)
    rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
    cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
    res["row_viol"] = float(np.maximum(np.maximum(rl - ax, ax - ru), 0).max()) if lp.num_row_ else 0.0
    res["col_viol"] = float(np.maximum(np.maximum(cl - x, x - cu), 0).max())
print("RESULT " + json.dumps(res))
"""


def highs_ref(files, cache):
    """HiGHS の目的関数値 (キャッシュがあれば読む)。"""
    path = cache / "highs_ref.json"
    ref = json.loads(path.read_text()) if path.exists() else {}
    missing = [f for f in files if os.path.basename(f) not in ref]
    if missing:
        import highspy

        for f in missing:
            h = highspy.Highs()
            h.setOptionValue("output_flag", False)
            h.readModel(f)
            h.run()
            ref[os.path.basename(f)] = {"status": str(h.getModelStatus()), "obj": h.getObjectiveValue()}
        path.write_text(json.dumps(ref, indent=1))
    return ref


def solve(mps, env_extra, timeout=300):
    env = dict(os.environ)
    env.update(env_extra)
    t0 = time.perf_counter()
    try:
        p = subprocess.run([sys.executable, "-c", WORKER, mps, str(REPO_ROOT / "scripts")], env=env, capture_output=True, text=True, timeout=timeout)
        res = next((json.loads(l[7:]) for l in p.stdout.splitlines() if l.startswith("RESULT ")), {"status": "crash", "obj": None})
    except subprocess.TimeoutExpired:
        res = {"status": "timeout", "obj": None}
    res["time"] = time.perf_counter() - t0
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--settings", default=",".join(SETTINGS), help="カンマ区切りの設定名")
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--cache-dir", type=Path, default=REPO_ROOT / ".netlib_cache")
    ap.add_argument("--cases", action="store_true", help="irish-electricity (旧前処理) と pilot87 (閾値 1e-3) も解く")
    ap.add_argument("--mitt-dir", type=Path, default=Path("/home/user/mitt"))
    args = ap.parse_args()

    files = sorted(glob.glob(str(args.cache_dir / "mps" / "*.mps")))
    ref = highs_ref(files, args.cache_dir)
    all_ok = True
    for name in [s for s in args.settings.split(",") if s]:
        env_extra = SETTINGS[name]
        with ThreadPoolExecutor(args.jobs) as ex:
            results = dict(zip(files, ex.map(lambda f: solve(f, env_extra), files)))
        bad = []
        for f, res in results.items():
            r = ref[os.path.basename(f)]
            ok = res["obj"] is not None and abs(res["obj"] - r["obj"]) <= 1e-6 * max(1.0, abs(r["obj"]))
            if not ok:
                bad.append((os.path.basename(f), res["status"]))
        all_ok &= not bad
        print(f"{name:5s} {len(files) - len(bad)}/{len(files)} optimal & match HiGHS; bad={bad}", flush=True)

    if args.cases:
        cases = [
            ("irish-electricity (旧前処理)", str(args.mitt_dir / "irish-electricity.mps"), {"ENOMOTO_T_PRESOLVE_FIXPOINT": "0", "ENOMOTO_T_LARGE_PRESOLVE_MIN_ROWS": "0", "ENOMOTO_INEQ_SINGLETON": "1"}, 2546254.5633092, 1800),
            ("pilot87 (閾値 1e-3)", str(args.cache_dir / "mps" / "pilot87.mps"), {"ENOMOTO_PIVOT_THRESHOLD": "1e-3"}, ref["pilot87.mps"]["obj"], 300),
        ]
        for label, mps, env_extra, z, timeout in cases:
            res = solve(mps, env_extra, timeout)
            rel = abs(res["obj"] - z) / max(1.0, abs(z)) if res["obj"] is not None else None
            ok = rel is not None and rel <= 1e-7
            all_ok &= ok
            print(f"{label}: {res}, rel={rel}", flush=True)
    sys.exit(0 if all_ok else 1)


if __name__ == "__main__":
    main()
