"""Mittelmann 186 問の 600 秒の比べ (2026-11-03): 分担表 shards.json の K 番目の問題を、ENOMOTO (今の既定、環境変数なし) と HiGHS (highspy) で
4 スレッド・600 秒で 1 回ずつ解き、res_sK.jsonl に足す。途中から再開できる。ソルバーは同時に 1 つ。問題ごとに順を入れ替える。
最大 RSS は 1 秒ごとに /proc/<pid>/status の VmRSS を読む。メモリ不足で殺された回は status "crash" (returncode つき)。
使い方: python analysis/mitt600_20261103/run600.py K"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = int(os.environ.get("MITT600_TL", "600"))  # 確かめ用に短くできる
k = int(sys.argv[1])
# MITT600_SHARDS (分担表のファイル名、既定 shards.json)・MITT600_TAG (結果の名前の頭、既定 res)・MITT600_SOLVERS (既定 ours,highs) で、
# 既定を変えた後に ENOMOTO だけを測り直す回 (2026-11-04 の res2、shards2.json) にも使う。
# MITT600_SOLVERS に "ours:<設定名>" を書くと、analysis/halve_submip_20261030/run.py の CFGS の環境変数を付けた ENOMOTO を回す (2026-11-07 の res4)
mine = json.loads((D / os.environ.get("MITT600_SHARDS", "shards.json")).read_text())["shards"][k]
OUT = D / f"{os.environ.get('MITT600_TAG', 'res')}_s{k}.jsonl"
SOLVERS = os.environ.get("MITT600_SOLVERS", "ours,highs").split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["solver"]))

def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0

def run(kind, mps):
    env = dict(os.environ)
    for key in list(env):
        if key.startswith("ENOMOTO_"): del env[key]
    if ":" in kind:  # "ours:<設定名>"
        kind, cfg = kind.split(":", 1)
        sys.path.insert(0, str(WT / "analysis" / "halve_submip_20261030"))
        from run import CFGS  # noqa: E402
        env.update(CFGS[cfg])
    env["ENOMOTO_MIP_THREADS"] = "4"
    if os.environ.get("MITT600_SEED") and kind == "ours":  # 2026-11-07 の res5: ENOMOTO の乱数の種 (既定の回も試作の回も同じ種)
        env["ENOMOTO_T_MIP_SEED"] = os.environ["MITT600_SEED"]
    t0 = time.time(); peak = 0.0
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps),
                           "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(1)
        if time.time() - t0 > TL + 300:
            pr.kill(); break
    out, err = pr.communicate()
    wall = time.time() - t0
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode, "error": err[-300:]}
    return res, round(peak), round(wall, 1)

for i, p in enumerate(mine):
    for attempt in range(5):
        try:
            mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e:  # noqa: BLE001
            err = e; time.sleep(30)
    else:
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "solver": "fetch", "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    order = SOLVERS if i % 2 == 0 else SOLVERS[::-1]
    for s in order:
        if (p, s) in done: continue
        res, peak, wall = run(s, mps)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "solver": s, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                                "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, s, res.get("status"), res.get("time"), peak, flush=True)
