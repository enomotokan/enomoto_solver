"""(A)・(δ)・(r)・裏の研磨 (i・w) を同じビルドで比べる (10 節): python verify_drw.py [分担 K 分担の数 N] → res_drw[_sK].jsonl に足す (途中から再開できる)。
6 問 × 上限 120・600 秒 × スレッド × 設定、種 1。外から上限 + 300 秒で止める。stderr の研磨の行 (POLISH_*) を取っておく。
- 設定 (どれも OVERRUN240=p、POLISH_STATS=v):
  - A = POLISH_PRESOLVE=1。8 節の (A)。
  - D = POLISH_PRESOLVE=d。(δ): 大きさで、前処理つきの研磨か、温めた研磨を選ぶ。
  - R = POLISH_PRESOLVE=r。2 スレッド以上で、前処理つきの研磨と温めた研磨を同時に回す。
  - Ri = R + POLISH_BG=i。待っている従が裏で研磨する。
  - Rw = R + POLISH_BG=w。働いている従も重みで研磨する。
- 120 秒: 1 スレッドは A・D、2・4・8 スレッドは A・D・R・Ri・Rw (8 スレッドは 4 コアでは割り当て過ぎ)。
- 600 秒: 1 スレッドは A・D、4 スレッドは A・D・Rw。
- 分担: 仕事の並びを N 個に分け、K 番目だけを回す (重い順に配る)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D_ = Path(__file__).resolve().parent
WT = D_.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["neos-5114902-kasavu", "neos-5049753-cuanza", "neos-873061", "thor50dday", "dano3_5", "snp-02-004-104"]
B = {"ENOMOTO_MIP_OVERRUN240": "p", "ENOMOTO_MIP_POLISH_STATS": "v"}
CFGS = {"A": {**B, "ENOMOTO_MIP_POLISH_PRESOLVE": "1"}, "D": {**B, "ENOMOTO_MIP_POLISH_PRESOLVE": "d"}, "R": {**B, "ENOMOTO_MIP_POLISH_PRESOLVE": "r"}}
CFGS["Ri"] = {**CFGS["R"], "ENOMOTO_MIP_POLISH_BG": "i"}
CFGS["Rw"] = {**CFGS["R"], "ENOMOTO_MIP_POLISH_BG": "w"}
JOBS = []
for tl, plan in ((120, {1: "A D", 2: "A D R Ri Rw", 4: "A D R Ri Rw", 8: "A D R Ri Rw"}), (600, {1: "A D", 4: "A D Rw"})):
    for p in PROBS:
        for th, cs in plan.items():
            for c in cs.split():
                JOBS.append((p, c, th, tl))
K, N = (int(sys.argv[1]), int(sys.argv[2])) if len(sys.argv) > 2 else (0, 1)
load = [0.0] * N; mine = []
for j in sorted(JOBS, key=lambda j: -j[3]):
    i = min(range(N), key=lambda i: load[i]); load[i] += j[3] + 20
    if i == K: mine.append(j)
OUT = D_ / (f"res_drw_s{K}.jsonl" if N > 1 else "res_drw.jsonl")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["threads"], r["tl"]))
print(f"仕事 {len(JOBS)} 回、分担 {K}/{N}: {len(mine)} 回、見積り {load[K] / 3600:.1f} 時間", flush=True)
for p, c, th, tl in mine:
    if (p, c, th, tl) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1", **CFGS[c]})
    t0 = time.time()
    pol = []
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
        res = [l for l in pr.stdout.splitlines() if l.startswith("{")]
        result = json.loads(res[-1]) if res else {"status": "crash"}
        pol = [l for l in pr.stderr.splitlines() if "POLISH" in l][-10:]
    except subprocess.TimeoutExpired:
        result = {"status": "killed"}
    row = {"problem": p, "cfg": c, "threads": th, "tl": tl, "wall": round(time.time() - t0, 1), "result": result, **({"polish": pol} if pol else {})}
    with open(OUT, "a") as f:
        f.write(json.dumps(row) + "\n")
    print(p, c, th, tl, result.get("status"), result.get("time"), result.get("objective"), row["wall"], flush=True)
