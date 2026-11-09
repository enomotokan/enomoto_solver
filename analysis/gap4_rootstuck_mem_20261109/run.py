"""GAP4 (2026-11-09): 根から出られない問題とメモリの計測。jobs.json の並び ([名前, 問題, ソルバー (ours/highs), 種, 秒, {環境変数}]) を順に回し、
runs.jsonl に結果・最大 RSS を足す。ログ (ENOMOTO_MIP_LOG=1 の標準エラー、HiGHS は ENOMOTO_BENCH_HIGHS_LOG=1 の標準出力) は logs/<名前>.log.gz。
smaps=1 の仕事は 5 秒ごとに /proc/<pid>/smaps_rollup と VmRSS を logs/<名前>.rss に書く。途中から再開できる。ソルバーは同時に 1 つ。
ログは書き終わるまで GAP4_TMP (/tmp/gap4_logs) に置く (追跡するファイルを rebase が作り直してログが切れるのを防ぐ)。
使い方: python analysis/gap4_rootstuck_mem_20261109/run.py jobs.json"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "runs.jsonl"
(D / "logs").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            done.add(json.loads(l)["name"])

def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0

TMP = Path(os.environ.get("GAP4_TMP", "/tmp/gap4_logs")); TMP.mkdir(parents=True, exist_ok=True)

def next_job():
    # 仕事の一覧は毎回読み直す (足せる)。D/PAUSE があれば消えるまで次を始めない (ビルドの間など)
    while (D / "PAUSE").exists():
        time.sleep(5)
    for j in json.loads((D / sys.argv[1]).read_text()):
        if j[0] not in done:
            return j
    return None

while (job := next_job()) is not None:
    name, p, solver, seed, tl, extra = job
    done.add(name)
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_BENCH_HIGHS_LOG": "1"})
    env.update(extra)
    logf = TMP / f"{name}.log"
    t0 = time.time(); peak = 0.0; tr = []
    with open(logf, "w") as lf:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps),
                               "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
        out_lines = []
        import threading
        th = threading.Thread(target=lambda: out_lines.extend(pr.stdout), daemon=True); th.start()
        k = 0
        while pr.poll() is None:
            r = rss_mb(pr.pid); peak = max(peak, r)
            if k % 5 == 0: tr.append((round(time.time() - t0), round(r)))
            k += 1
            time.sleep(1)
            if time.time() - t0 > tl + 300:
                pr.kill(); break
        th.join(5)
    out = "".join(out_lines)
    with open(logf, "a") as lf:
        lf.write("\n#STDOUT\n" + out)
    try:
        res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    with open(logf, "rb") as a, gzip.open(str(D / "logs" / f"{name}.log.gz"), "wb") as b:
        b.write(a.read())
    logf.unlink()
    (D / "logs" / f"{name}.rss").write_text("\n".join(f"{a} {b}" for a, b in tr))
    with open(OUT, "a") as f:
        f.write(json.dumps({"name": name, "problem": p, "solver": solver, "seed": seed, "tl": tl, "env": extra, "result": res,
                            "max_rss_mb": round(peak), "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(name, res.get("status"), res.get("time"), round(peak), flush=True)
