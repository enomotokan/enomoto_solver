"""(2) の分担 K を回す: shards_gap1u.json の K 番目の (問題, 設定, 種) を 4 スレッド・300 秒で 1 回ずつ解き、gap1u_sK.jsonl に足す。
途中から再開できる。ソルバーは同時に 1 つ。行の形は drcr_20261110/run.py と同じ (+ "shard")。使い方: python run_gap1u.py K [表の名前]
表の名前 (既定 gap1u) を NAME とすると shards_NAME.json を読み、NAME_sK.jsonl に足す (fu58 = 58 問の対の計測、plan_fu58.py)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "fu": {"ENOMOTO_MIP_GAP1": "u"},
        "fui": {"ENOMOTO_MIP_GAP1": "u", "ENOMOTO_MIP_FOLD_PROBE_INT": "1"},
        "nfu": {"ENOMOTO_MIP_FOLD": "0", "ENOMOTO_MIP_GAP1": "u"}}
k = int(sys.argv[1])
NAME = sys.argv[2] if len(sys.argv) > 2 else "gap1u"
plan = json.loads((D / f"shards_{NAME}.json").read_text())
TL = plan["tl"]
OUT = D / f"{NAME}_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for p, c, seed in plan["shards"][k]:
    if (p, c, seed) in done:
        continue
    for attempt in range(5):
        try:
            mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e:  # noqa: BLE001
            err = e; time.sleep(30)
    else:
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
    t0 = time.time(); fold = []
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                            "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 400)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        fold = [l for l in r.stderr.splitlines() if l.startswith("MIP_FOLD")][:6]
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": str(e)[:300]}
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": TL, "result": res, "fold": fold,
                            "wall": round(time.time() - t0, 1), "shard": k, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), flush=True)
