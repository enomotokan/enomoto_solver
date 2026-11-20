"""M の各問題で SUBWAIT (sw2) の打ち切りが働くかを、外側の根の段だけ回して判定する (README 16 節)。
python swprobe.py [種]。sw2・4 スレッドでログつきで解き、最初の「TRSTAT: tree start」(木に入った) か 600 秒で止める。
結果は swprobe.jsonl に 1 問 1 行 (根の段の秒、サブ MIP の始まり・終わり、SUBWAIT の行)。ソルバーは同時に 1 つ。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
seed = sys.argv[1] if len(sys.argv) > 1 else "1"
probs = sorted({t[0] for s in json.loads((D / "shards_MSW2.json").read_text())["shards"] for t in s})
OUT = D / "swprobe.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in probs:
    if p in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_LOG": "1", "ENOMOTO_T_MIP_SUBWAIT": "2"})
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines, tree, sw, subs = [], None, [], []
    for l in pr.stdout:
        t = time.time() - t0
        lines.append(f"[{t:7.2f}] {l}")
        if "SUBWAIT" in l:
            sw.append(l.strip())
        if "parallel sub-MIP" in l:
            subs.append(f"{t:.1f} {l.strip()[:160]}")
        if "TRSTAT: tree start" in l:
            tree = t
            break
    pr.kill(); pr.wait()
    (D / "logs" / f"{p}_swprobe_s{seed}.log").write_text("".join(lines[-400:]))
    row = {"problem": p, "seed": seed, "tree_start": tree, "wall": round(time.time() - t0, 1), "subwait": sw, "submips": subs}
    with open(OUT, "a") as f:
        f.write(json.dumps(row) + "\n")
    print(p, tree, len(sw), flush=True)
