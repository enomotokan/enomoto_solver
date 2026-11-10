"""cut-based の衝突解析の制約ごとの妥当性の検査 (fuzz_cutconf.py の問題で): 問題を MPS に書き、ENOMOTO の前処理の後の問題を
書き出し (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3)、HiGHS でその最適解を求めてデバッグ解にし、ENOMOTO_MIP_CUTCONF_CHECK
(どの実行可能解にも妥当な材料だけを使う) で解いて、作った制約がデバッグ解を切ったら (MIP_DEBUG_SOL: cutconf) 数える。
使い方: python fuzz_check.py 問題数 [開始seed] [追加の環境変数=値 ...]"""
import os, subprocess, sys, tempfile, time
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
src = open(D / "fuzz_cutconf.py").read().split('if __name__ != "__main__":')[0]
ns = {"__file__": str(D / "fuzz_cutconf.py")}
exec(compile(src, "fuzz_cutconf.py", "exec"), ns)
gen = ns["gen"]
inf = float("inf")
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
extra = dict(a.split("=", 1) for a in sys.argv[3:])
big = extra.pop("BIG", "1") == "1"
tmp = Path(tempfile.mkdtemp())


def write_mps(lp, path):
    lo, up, c, it, rows, rlo, rup = lp
    h = highspy.Highs(); h.setOptionValue("output_flag", False)
    K = highspy.kHighsInf
    for j in range(len(lo)):
        h.addVar(lo[j] if lo[j] > -inf else -K, up[j] if up[j] < inf else K); h.changeColCost(j, c[j])
        if it[j]: h.changeColIntegrality(j, highspy.HighsVarType.kInteger)
    for r, l, u in zip(rows, rlo, rup):
        h.addRow(l if l > -inf else -K, u if u < inf else K, len(r), [a for a, _ in r], [b for _, b in r])
    h.writeModel(str(path))


def worker(mps, env_add, tl=20):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "2"}); env.update(env_add)
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "2"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 3 + 60)
    return pr.stdout, pr.stderr


bad = skipped = outs = viol_runs = 0
t0 = time.time()
for s in range(start, start + N):
    lp = gen(s, big)
    mps = tmp / f"p{s}.mps"; pre = tmp / f"p{s}_pre.mps"; sol = tmp / f"p{s}.sol"
    write_mps(lp, mps)
    worker(mps, {"ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(pre), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
    if not pre.exists():
        skipped += 1; continue
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 1e-9); h.readModel(str(pre)); h.run()
    if h.getModelStatus() != highspy.HighsModelStatus.kOptimal:
        skipped += 1; continue
    sol.write_text("\n".join(repr(float(v)) for v in h.getSolution().col_value) + "\n")
    env = {"ENOMOTO_MIP_DEBUG_SOL": str(sol), "ENOMOTO_MIP_CUTCONF": "1", "ENOMOTO_MIP_CUTCONF_CHECK": "1", "ENOMOTO_MIP_NO_SYMMETRY": "1"}
    env.update(extra)
    out, err = worker(mps, env)
    lines = err.splitlines()
    v = [l for l in lines if l.startswith("MIP_DEBUG_SOL: cutconf")]
    feas = [l for l in lines if "MIP_DEBUG_SOL: reduced objective" in l]
    cc = [l for l in lines if l.startswith("MIP: cutconf")]
    o = int(cc[-1].split(" out ")[1].split()[0]) if cc else 0
    outs += o
    if any("feasible false" in l for l in feas):
        skipped += 1; continue
    print("seed", s, "cutconf", o, (cc[-1][28:200] if cc else ""), flush=True)
    if v:
        bad += 1; viol_runs += len(v)
        print("BAD seed", s, len(v), v[0][:300], flush=True)
print("bad", bad, "of", N, "skipped", skipped, "cutconf constraints", outs, "time %.1f" % (time.time() - t0))
