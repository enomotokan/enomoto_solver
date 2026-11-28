"""裏の研磨 POLISH_BG=i の確かめ (verify.py の写し): PYTHONPATH=/home/user/es_bg/python python verify_bg.py → res_bg.jsonl に足す (途中から再開できる)。
6 問 × 2・4・8 スレッド × 上限 120 秒 × {ppA (POLISH_PRESOLVE=1 + OVERRUN240=p), bgA (ppA + POLISH_BG=i)}、種 1。8 スレッドは 4 コアでは割り当て過ぎなので動作の確かめだけ。
以下は verify.py の説明:
6 問 (時間切れになる問題) × 1・4 スレッド × 上限 120・600 秒 × 既定・`ENOMOTO_MIP_OVERRUN240=p`、種 1。上限 120 秒の回を先に全部回す。
外から上限 + 300 秒で止める (止めたら status killed)。見るもの: 上限を守れたか (wall・result.time)、返した目的値の違い。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["neos-5114902-kasavu", "neos-5049753-cuanza", "neos-873061", "thor50dday", "dano3_5", "snp-02-004-104"]
V = {"ENOMOTO_MIP_POLISH_PRESOLVE": "1", "ENOMOTO_MIP_OVERRUN240": "p", "ENOMOTO_MIP_POLISH_STATS": "v"}
CFGS = {"ppA": V, "bgA": {**V, "ENOMOTO_MIP_POLISH_BG": "i"}}
OUT = D / "res_bg.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["threads"], r["tl"]))
for tl in (120,):
    for p in PROBS:
        for th in (2, 4, 8):
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
                    pol = [l for l in pr.stderr.splitlines() if "POLISH" in l][-8:]
                except subprocess.TimeoutExpired:
                    result = {"status": "killed"}
                    pol = []
                row = {"problem": p, "cfg": c, "threads": th, "tl": tl, "wall": round(time.time() - t0, 1), "result": result, **({"polish": pol} if pol else {})}
                with open(OUT, "a") as f:
                    f.write(json.dumps(row) + "\n")
                print(p, c, th, tl, result.get("status"), result.get("time"), result.get("objective"), row["wall"], flush=True)
