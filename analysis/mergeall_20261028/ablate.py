"""B の切り分け (悪くなった 10 問、A / A+双子 / A+LB2・PROX2)。全ブランチの取り込み後の試作の既定化の確かめ (2026-10-28)。16 スレッド・60 秒、負けている 21 問 + MIPLIB 40 問 (重複を除き 58 問)、2 回。
1 問ごとに 3 設定を続けて解き、設定の順は回ごとに回す (同じ問題の比較を近い時刻で)。終わった (問題, 設定, 回) は飛ばして再開できる。
結果は analysis/mergeall_20261028/ablate.jsonl。WMI で切り離して起動する。"""
import ctypes, json, os, subprocess, sys, time
from pathlib import Path

WT = Path(__file__).resolve().parents[2]
REPO = Path(r"C:\Users\enomo\Desktop\2026\python\ENOMOTO-Solver")
PY = str(REPO / ".tmpx" / "bench_venv" / "Scripts" / "python.exe")
OUT = WT / "analysis" / "mergeall_20261028" / "ablate.jsonl"
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb

loss = (REPO / ".tmpx" / "loss_names.txt").read_text().split()
PROBS = "neos-911970 h80x6320d qiu nw04 eil33-2 drayage-100-23 pk1 noswot mc11 neos-960392".split()
A = {"ENOMOTO_MIP_SEPA_MCF": "auto", "ENOMOTO_MIP_SYM_OF": "1", "ENOMOTO_MIP_SYM_OF_SCHREIER": "1",
     "ENOMOTO_PRESOLVE_PARALLEL_ALL_KEPT": "1", "ENOMOTO_PRESOLVE_SLACK_COL": "1",
     "ENOMOTO_MIP_TWOOPT": "1", "ENOMOTO_MIP_SHIFT_HEURS": "root"}
B = dict(A, ENOMOTO_MIP_SYM_TWINS="1", ENOMOTO_PRESOLVE_PARALLEL_INT="1",
         ENOMOTO_MIP_LB2="1", ENOMOTO_MIP_PROX2="1", ENOMOTO_MIP_HSUB_STALL="1")
TW = dict(A, ENOMOTO_MIP_SYM_TWINS="1", ENOMOTO_PRESOLVE_PARALLEL_INT="1")
LN = dict(A, ENOMOTO_MIP_LB2="1", ENOMOTO_MIP_PROX2="1", ENOMOTO_MIP_HSUB_STALL="1")
CFGS = {"A": A, "A+twins": TW, "A+lns2": LN}
ORDERS = [["A", "A+twins", "A+lns2"], ["A+lns2", "A+twins", "A"]]

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
