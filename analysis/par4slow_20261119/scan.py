"""M の問題で、前処理の LP による固定の最初の LP の手間・秒・基底を渡すかを見る: python scan.py スレッド数 秒 [問題 ...]。
既定の設定で ENOMOTO_MIP_LOG=1 をつけて「秒」で打ち切り、"LP probing: root LP" の行を scan_tT.jsonl に足す (途中から再開できる)。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
th, tl = sys.argv[1], sys.argv[2]
probs = sys.argv[3:] or sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
out = D / f"scan_t{th}.jsonl"
done = {json.loads(l)["problem"] for l in out.read_text().splitlines()} if out.exists() else set()
pat = re.compile(r"LP probing: root LP ([\d.]+)s \((\d+) iters, size (\d+)\), tried (\d+), fixed (\d+), basis passed (\w+)")
for p in probs:
    if p in done: continue
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        rec = {"problem": p, "error": str(e)[:100]}
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 3 + 60)
            err = r.stderr
        except subprocess.TimeoutExpired as e:
            err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
        m = pat.search(err)
        rec = {"problem": p}
        if m:
            s, it, sz, tr, fx, ps = m.groups()
            rec.update({"secs": float(s), "iters": int(it), "size": int(sz), "work": int(it) * int(sz), "fixed": int(fx), "passed": ps == "true"})
        else:
            rec["probe"] = None
    with open(out, "a") as f: f.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
