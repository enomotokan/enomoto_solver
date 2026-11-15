"""GAP4mem (2026-11-11): tbfp-network で rocn の害の元を分ける。既定と ENOMOTO_MIP_GAP4 の 1 文字 (r・o・n・c) を種 1・2 で 4 スレッド・600 秒、
分担表 shards_split.json の K 番目の (問題, 設定, 種) を順に解き、res_split_sK.jsonl に足す (最大 RSS・TRSTAT の木に入った時刻つき)。途中から再開できる。
ソルバーは同時に 1 つ。wheel はインストール済みのもの (固定のコミットでビルドしたもの) を使う。
使い方: python analysis/gap4mem_20261111/run_split.py K"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "r": {"ENOMOTO_MIP_GAP4": "r"}, "o": {"ENOMOTO_MIP_GAP4": "o"}, "n": {"ENOMOTO_MIP_GAP4": "n"}, "c": {"ENOMOTO_MIP_GAP4": "c"},
        "rocn": {"ENOMOTO_MIP_GAP4": "rocn"}}
TL = 600
k = int(sys.argv[1])
mine = json.loads((D / "shards_split.json").read_text())["shards"][k]
OUT = D / f"res_split_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))

def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0

for p, c, seed in mine:
    if (p, c, seed) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[c]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1")
    t0 = time.time(); peak = 0.0
    errf = open(f"/tmp/gap4split_{k}.log", "w")
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=errf, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(1)
        if time.time() - t0 > TL + 300:
            pr.kill(); break
    out, _ = pr.communicate(); errf.close()
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    tree = None
    for l in open(f"/tmp/gap4split_{k}.log", errors="replace"):
        m = re.search(r"TRSTAT: tree start outer ([0-9.]+)", l)
        if m: tree = float(m.group(1))
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": res, "max_rss_mb": round(peak), "tree_start_outer": tree,
                            "wall": round(time.time() - t0, 1), "shard": k, "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), round(peak), tree, flush=True)
