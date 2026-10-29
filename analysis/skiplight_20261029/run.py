"""軽いヒューリスティクスの解では早めの再スタートをしない形 (skip) と、早めの再スタートの閾値を 0.05 に上げる形 (frac05) を既定 (new) と比べる (2026-10-29)。
16 スレッド・60 秒、負けている 21 問 + MIPLIB 40 問 (58 問)、2 回、設定の順は回ごとに逆。WMI で切り離して起動する。"""
import ctypes, json, os, subprocess, sys, time
from pathlib import Path

WT = Path(__file__).resolve().parents[2]
REPO = Path(r"C:\Users\enomo\Desktop\2026\python\ENOMOTO-Solver")
PY = str(REPO / ".tmpx" / "bench_venv" / "Scripts" / "python.exe")
OUT = WT / "analysis" / "skiplight_20261029" / "ab.jsonl"
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb

loss = (REPO / ".tmpx" / "loss_names.txt").read_text().split()
PROBS = loss + [p for p in rb.PROBLEMS if p not in loss]
CFGS = {"new": {}, "skip": {"ENOMOTO_MIP_EARLY_RESTART_SKIP_LIGHT": "1"},
        "frac05": {"ENOMOTO_T_MIP_EARLY_RESTART_FRAC": "0.05"}}
ORDERS = [["new", "skip", "frac05"], ["frac05", "skip", "new"]]

ctypes.windll.kernel32.SetThreadExecutionState(0x80000000 | 0x00000001)
done = set()
if OUT.exists():
    for l in OUT.read_text(encoding="utf-8").splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
for rep in (0, 1):
    for p in PROBS:
        for cfg in ORDERS[rep]:
            if (p, cfg, rep) in done:
                continue
            mps = rb.fetch(p, REPO / ".miplib_cache")
            env = dict(os.environ, PYTHONUTF8="1", ENOMOTO_MIP_THREADS="16", PYTHONPATH=str(WT / "python"), **CFGS[cfg])
            t0 = time.time()
            try:
                r = subprocess.run([PY, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", "60", "--threads", "16"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(OUT, "a", encoding="utf-8") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "wall": time.time() - t0, "result": res}) + "\n")
OUT.with_suffix(".done").write_text(time.strftime("%Y-%m-%d %H:%M:%S"))
