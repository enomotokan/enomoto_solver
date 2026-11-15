"""OBJINT の `c` が付ける範囲 (`MIP: OBJINT c: lattice ...` の最初の行) と、`OBJINT=1` の刻み (`objective step with implied integers`) を
88 問で並べる → detect_c.jsonl。5 秒だけ解く (行は求解の始めに出る)。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "implint_tu_20261110"))
from probs_all import ALL  # noqa: E402
OUT = D / "detect_c.jsonl"
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
for p in ALL + ["csched007"]:
    if p in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="2", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_OBJINT="hfc")
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                        "--time-limit", "5", "--threads", "2"], cwd=WT, env=env, capture_output=True, text=True, timeout=900)
    out = r.stdout + r.stderr
    lat = [l for l in out.splitlines() if "OBJINT c:" in l]
    st = [l for l in out.splitlines() if "objective step with implied integers" in l]
    with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "lat": lat[:3], "step": st[:1]}) + "\n")
    print(p, (lat[:1] or [""])[0][:150], flush=True)
