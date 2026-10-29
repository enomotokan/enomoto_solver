"""CBC との比較から作った試作 (既定では使わない環境変数) を既定 (base) と比べる (2026-10-29)。
4 スレッド・60 秒、負けている 21 問 + MIPLIB 40 問 (58 問)。問題ごとに設定を順に回し、回ごとに順を逆にする。
使い方: python run.py <出力の jsonl> <回の数> <設定名> [<設定名> ...]  (設定名は CFGS のキー。base を含める)
"""
import json, os, subprocess, sys, time
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from probs import PROBS  # noqa: E402
from cfgs import CFGS  # noqa: E402

TL, TH = 60, 4
out = Path(sys.argv[1])
reps = int(sys.argv[2])
names = sys.argv[3:]
probs = os.environ.get("CBCIMPL_PROBS", "").split() or PROBS
done = set()
if out.exists():
    for l in out.read_text(encoding="utf-8").splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["cfg"], r["rep"]))
for rep in range(reps):
    order = names if rep % 2 == 0 else names[::-1]
    for p in probs:
        for cfg in order:
            if (p, cfg, rep) in done:
                continue
            mps = WT / ".miplib_cache" / f"{p}.mps"
            env = dict(os.environ, PYTHONPATH=str(WT / "python"), **CFGS[cfg])
            t0 = time.time()
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(TL), "--threads", str(TH)], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a", encoding="utf-8") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "wall": time.time() - t0, "result": res}) + "\n")
