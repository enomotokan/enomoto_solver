"""CLQPRB の続き (幅 1 の整数・暗黙の整数の数え上げ): 診断 ENOMOTO_MIP_DEBUG_INTCENSUS の INTCENSUS 行を集める。
使い方: python census.py 問題の一覧 K N [設定]  (一覧の K 番目から N 個おき。設定 def (既定) か clq (ENOMOTO_MIP_CLQPRB=1))
4 スレッド・種 1 で回し、根の後の行 (c_root_r0) が出るか 150 秒で止める。出力: census_<一覧の名前>_<設定>_s<K>.jsonl (問題ごとに 1 行)。
途中から再開できる。前もって: sh build.sh"""
import json, os, re, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
lst, k, nk = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
cfg = sys.argv[4] if len(sys.argv) > 4 else "def"
probs = [l.strip() for l in open(D / lst) if l.strip()][k::nk]
out = D / f"census_{Path(lst).stem}_{cfg}_s{k}.jsonl"
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
TL = 150
for p in probs:
    if p in done: continue
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        with open(out, "a") as f: f.write(json.dumps({"problem": p, "error": str(e)[:200]}) + "\n")
        continue
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_INTCENSUS="1")
    if cfg == "clq": env["ENOMOTO_MIP_CLQPRB"] = "1"
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    killer = threading.Timer(TL + 60, pr.kill); killer.start()
    res = {"problem": p, "cfg": cfg, "census": []}
    for line in pr.stdout:
        if line.startswith("INTCENSUS "):
            c = json.loads(line[10:]); c["t"] = round(time.time() - t0, 1); res["census"].append(c)
            if c["at"].startswith("c_root"): break
        elif line.startswith("{"):
            try: res["final"] = json.loads(line)
            except Exception: pass  # noqa: BLE001
    killer.cancel(); pr.kill(); pr.wait()
    res["wall"] = round(time.time() - t0, 1)
    with open(out, "a") as f: f.write(json.dumps(res) + "\n")
    print(p, [c["at"] for c in res["census"]], res["wall"], flush=True)
