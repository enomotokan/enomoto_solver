"""ログつきで順に解く: python batch.py 名前 (jobs_名前.json の [問題, 設定, 種, 秒, 追加の引数...] を順に)。
ログは logs/名前/問題.設定.s種.log.gz、結果は res_名前.jsonl (RESULT 行)。途中から再開できる。ソルバーは同時に 1 つ、4 スレッド。"""
import gzip, json, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
name = sys.argv[1]
jobs = json.loads((D / f"jobs_{name}.json").read_text())
L = D / "logs" / name; L.mkdir(parents=True, exist_ok=True)
OUT = D / f"res_{name}.jsonl"
done = set()
if OUT.exists():
    done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines()) if r}
for p, cfg, seed, tl, *extra in jobs:
    if (p, cfg, seed) in done: continue
    log = L / f"{p}.{cfg}.s{seed}.log.gz"
    t0 = time.time()
    try:
        subprocess.run([sys.executable, str(D / "run1.py"), p, cfg, str(seed), "4", str(tl), str(log)] + extra, check=False)
        last = [l for l in gzip.open(log, "rt", errors="replace").read().splitlines() if l.startswith("RESULT")]
        res = last[-1].split()[1:] if last else ["crash"]
    except Exception as e:  # noqa: BLE001
        res = ["crash", str(e)[:100]]
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": tl, "extra": extra, "res": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, res, flush=True)
