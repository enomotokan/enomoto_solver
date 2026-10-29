"""既定にした 7 つの試作を 1 つずつ外す切り分け (2026-10-29)。新しい既定で落ちた 3 問 x 9 設定 x 4 回、16 スレッド・60 秒。
1 問ごとに 9 設定を続けて解き、設定の順は回ごとにずらす。終わった組は飛ばして再開できる。WMI で切り離して起動する。"""
import ctypes, json, os, subprocess, sys, time
from pathlib import Path

WT = Path(__file__).resolve().parents[2]
REPO = Path(r"C:\Users\enomo\Desktop\2026\python\ENOMOTO-Solver")
PY = str(REPO / ".tmpx" / "bench_venv" / "Scripts" / "python.exe")
OUT = WT / "analysis" / "ablate7_20261029" / "ab.jsonl"
PROBS = ["neos-960392", "h80x6320d", "timtab1"]
OFF = {"mcf": {"ENOMOTO_MIP_SEPA_MCF": "0"}, "sym_of": {"ENOMOTO_MIP_NO_SYM_OF": "1"},
       "schreier": {"ENOMOTO_MIP_NO_SYM_OF_SCHREIER": "1"}, "all_kept": {"ENOMOTO_PRESOLVE_PARALLEL_NO_ALL_KEPT": "1"},
       "slack": {"ENOMOTO_PRESOLVE_NO_SLACK_COL": "1"}, "twoopt": {"ENOMOTO_MIP_TWOOPT": "0"}, "shift": {"ENOMOTO_MIP_SHIFT_HEURS": "0"}}
CFGS = {"new": {}, **{f"-{k}": v for k, v in OFF.items()}, "old": {k: v for d in OFF.values() for k, v in d.items()}}
names = list(CFGS)

ctypes.windll.kernel32.SetThreadExecutionState(0x80000000 | 0x00000001)
done = set()
if OUT.exists():
    for l in OUT.read_text(encoding="utf-8").splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
for rep in range(4):
    order = names[rep * 2 % len(names):] + names[:rep * 2 % len(names)]
    if rep % 2:
        order = order[::-1]
    for p in PROBS:
        for cfg in order:
            if (p, cfg, rep) in done:
                continue
            env = dict(os.environ, PYTHONUTF8="1", ENOMOTO_MIP_THREADS="16", PYTHONPATH=str(WT / "python"), **CFGS[cfg])
            t0 = time.time()
            try:
                r = subprocess.run([PY, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(REPO / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "16"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(OUT, "a", encoding="utf-8") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "wall": time.time() - t0, "result": res}) + "\n")
OUT.with_suffix(".done").write_text(time.strftime("%Y-%m-%d %H:%M:%S"))
