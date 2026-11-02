"""試作 ENOMOTO_SIMPLEX_WARM_SING_REPAIR=1 (rep) を既定 (base) と比べる (2026-11-02)。

4 スレッド・60 秒、ソルバーは同時に 1 つ。問題ごとに設定を順に回し、回ごとに順を逆にする。どちらの設定も
ENOMOTO_MIP_NB_DIAG=1 を付け、NB_DIAG の行 (場所ごとの LP 数・反復・特異で捨てた LP・差し替えた LP) を結果と一緒に jsonl に書く。
既にある (問題, 設定, 回) は飛ばすので、中断しても同じコマンドで再開できる。

使い方: python run.py <出力の jsonl> <回の数> [--probs P ...] [--cfgs base rep]
"""
import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
import probs as P  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402

CFGS = {"base": {}, "rep": {"ENOMOTO_SIMPLEX_WARM_SING_REPAIR": "1"}}
SEL10 = ["ns1116954", "mzzv42z", "neos5", "misc07", "noswot", "neos-911970", "neos-960392", "physiciansched6-2", "neos-827175", "swath1"]

ap = argparse.ArgumentParser()
ap.add_argument("out")
ap.add_argument("reps", type=int)
ap.add_argument("--probs", nargs="*")
ap.add_argument("--all58", action="store_true")
ap.add_argument("--cfgs", nargs="*", default=["base", "rep"])
ap.add_argument("--time-limit", type=float, default=60.0)
ap.add_argument("--threads", type=int, default=4)
a = ap.parse_args()
probs = P.PROBS if a.all58 else (a.probs or SEL10)
out = Path(a.out)
done = set()
if out.exists():
    for l in out.read_text(encoding="utf-8").splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
for rep in range(a.reps):
    order = a.cfgs if rep % 2 == 0 else a.cfgs[::-1]
    for p in probs:
        for cfg in order:
            if (p, cfg, rep) in done:
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = dict(os.environ, ENOMOTO_MIP_THREADS=str(a.threads), ENOMOTO_MIP_NB_DIAG="1", **CFGS[cfg])
            t0 = time.time()
            diag = []
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(a.time_limit), "--threads", str(a.threads)], cwd=WT, env=env,
                                   capture_output=True, text=True, timeout=a.time_limit * 1.5 + 60)
                diag = [l for l in r.stderr.splitlines() if l.startswith("NB_DIAG")]
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a", encoding="utf-8") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "wall": time.time() - t0, "result": res, "diag": diag}) + "\n")
            print(p, cfg, rep, res.get("status"), res.get("time"), flush=True)
