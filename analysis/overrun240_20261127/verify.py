"""試作 (p) の確かめ: python verify.py → res_v.jsonl に足す (途中から再開できる)。
6 問 (時間切れになる問題) × 1・4 スレッド × 上限 120・600 秒 × 既定・`ENOMOTO_MIP_OVERRUN240=p`、種 1。上限 120 秒の回を先に全部回す。
外から上限 + 300 秒で止める (止めたら status killed)。見るもの: 上限を守れたか (wall・result.time)、返した目的値の違い。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["neos-5114902-kasavu", "neos-5049753-cuanza", "neos-873061", "thor50dday", "dano3_5", "snp-02-004-104"]
CFGS = {"def": {}, "p": {"ENOMOTO_MIP_OVERRUN240": "p"}}
OUT = D / "res_v.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["threads"], r["tl"]))
for tl in (120, 600):
    for p in PROBS:
        for th in (1, 4):
            for c, ev in CFGS.items():
                if (p, c, th, tl) in done:
                    continue
                mps = rb.fetch(p, WT / ".miplib_cache")
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1", **ev})
                t0 = time.time()
                try:
                    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
                    res = [l for l in pr.stdout.splitlines() if l.startswith("{")]
                    result = json.loads(res[-1]) if res else {"status": "crash"}
                except subprocess.TimeoutExpired:
                    result = {"status": "killed"}
                row = {"problem": p, "cfg": c, "threads": th, "tl": tl, "wall": round(time.time() - t0, 1), "result": result}
                with open(OUT, "a") as f:
                    f.write(json.dumps(row) + "\n")
                print(p, c, th, tl, result.get("status"), result.get("time"), result.get("objective"), row["wall"], flush=True)
