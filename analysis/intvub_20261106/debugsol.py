"""デバッグ解による確かめ (analysis/intvub_20261106.md の 2): 前処理後の模型を書き出し (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3)、HiGHS で
最適解を求め (mip_rel_gap 0)、それを ENOMOTO_MIP_DEBUG_SOL に渡して試作で解き、分離の候補がデバッグ解を切るかを
ENOMOTO_MIP_INTVUB_CHECK で数える。結果は debugsol.txt に足す。使い方: python debugsol.py 問題 設定(iv1|iv3) [時間]"""
import os, re, subprocess, sys
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
p, cfg = sys.argv[1], sys.argv[2]
tl = sys.argv[3] if len(sys.argv) > 3 else "60"
mps = WT / ".miplib_cache" / f"{p}.mps"
pre, sol = Path(f"/tmp/ds/{p}_pre.mps"), Path(f"/tmp/ds/{p}_pre.sol")
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
base.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
run = lambda env: subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"],
                                 cwd=WT, env=env, capture_output=True, text=True, timeout=int(tl) + 600)
if not sol.exists():
    run({**base, "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(pre), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1"})
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("mip_rel_gap", 0.0); h.setOptionValue("time_limit", 1200.0)
    h.readModel(str(pre)); h.run()
    st = h.modelStatusToString(h.getModelStatus())
    if st != "Optimal":
        print(p, "HiGHS", st); sys.exit(1)
    sol.write_text("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")
env = {**base, "ENOMOTO_MIP_DEBUG_SOL": str(sol), "ENOMOTO_MIP_INTVUB_CHECK": "1", "ENOMOTO_MIP_INTVUB_LOG": "1"}
env.update({"iv1": {"ENOMOTO_MIP_INTVUB": "1"}, "iv3": {"ENOMOTO_MIP_INTVUB": "3"}, "def": {}}[cfg])
r = run(env)
err = r.stderr
feas = re.findall(r"MIP_DEBUG_SOL: reduced objective (\S+) feasible (\S+)", err)
chk = re.findall(r"INTVUB_CHECK: candidates checked (\d+) violated (\d+) \(debug solution within global bounds\) (\d+) \(outside\)", err)
# スレッドごとの累計の最大を合計の目安にする (各スレッドの最後の行)
viol_lines = [l for l in err.splitlines() if "INTVUB_CHECK: candidate cut" in l]
other = [l for l in err.splitlines() if l.startswith("MIP_DEBUG_SOL") and "INTVUB_CHECK" not in l and "reduced objective" not in l]
res = [l for l in r.stdout.splitlines() if l.startswith("{")][-1:]
line = f"{p} {cfg}: debug sol {feas[:1]}, check lines {len(chk)}, max checked {max((int(a) for a, _, _ in chk), default=0)}, violated(in bounds) lines {len(viol_lines)}, max violated(outside) {max((int(c) for _, _, c in chk), default=0)}, other MIP_DEBUG_SOL lines {len(other)} {other[:2]}, {res[0][:160] if res else ''}"
print(line)
with open(D / "debugsol.txt", "a") as f:
    f.write(line + "\n")
