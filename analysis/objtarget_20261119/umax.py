"""OBJTARGET 7: LP 緩和の最小の下界 L と最大の上界 U_max (`ENOMOTO_MIP_TPRE=u` の `MIP: TPRE_UMAX` の行) を集める。
python umax.py 問題... → res_umax.jsonl に足す。前処理の後で止める (`ENOMOTO_MIP_DEBUG_PRETIME=stop`)。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "res_umax.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in sys.argv[1:]:
    if p in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_TPRE": "u", "ENOMOTO_MIP_DEBUG_PRETIME": "stop", "ENOMOTO_MIP_THREADS": "4"})
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=900)
        ls = [l for l in pr.stderr.splitlines() if "TPRE_UMAX" in l]
    except subprocess.TimeoutExpired:
        ls = []
    row = {"problem": p, "line": ls[-1] if ls else None, "wall": round(time.time() - t0, 1)}
    if ls:
        caps = {}
        for m in re.finditer(r"it(\d+) L ([-+0-9.einfINFNa]+) Umax ([-+0-9.einfINFNa]+)", ls[-1]):
            caps[int(m.group(1))] = (float(m.group(2)), float(m.group(3)))
        row["caps"] = caps
        row["secs"] = [float(x) for x in re.findall(r"secs_m(?:in|ax) ([0-9.]+)", ls[-1])]
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, row.get("caps", {}).get("20000") if "caps" in row else None, row["wall"], flush=True)
