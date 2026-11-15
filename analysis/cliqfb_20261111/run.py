"""CLIQFB の計測 (analysis/cliqfb_20261111.md): 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足す (途中から再開できる。
同じ (問題, 種) の設定は続けて回し、設定の順は種ごとに逆)。種は ENOMOTO_T_MIP_SEED=種。計測する wheel は環境変数 CF_PKG
(pip install --target の場所) を PYTHONPATH の先に置いて固定する (既定と試作は同じ wheel で、環境変数だけが違う)。
stderr の `MIP: clique probe`・`MIP: clique fallback` の行も記録する (ENOMOTO_MIP_LOG=1 のときだけ出る。`CF_LOG=1` で付ける。
`CF_ENV=K=V,...` で全部の設定に環境変数を足す (例 ENOMOTO_MIP_NODE_LIMIT=1)。`CF_LOGDIR=<dir>` なら stderr 全体を <dir>/<問題>.<設定>.s<種>.log.gz に残す)。
使い方: python run.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...|@58|@cq|@k<i>/<n>>
  @k<i>/<n>: 58 問を n 個に分けた i 番目 (0 始まり)。同じ (問題, 種) の既定と試作は同じ分担に入る"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS as P58  # noqa: E402
CFGS = {
    "def": {},                                                      # 既定 (fallback)
    "cp": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe"},                     # 試作: 始めの 5 ラウンドのあり・なしの比べ
    "cps": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe_s"},                   # 試作: cp の余分を減らす形 (破れるクリークがなければ・選ばれなければ比べない)
    "cpr": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe_r"},                   # 試作: cps に加え、使わないときは比べの状態を捨てて既定と同じ道筋にする
    "cpf": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe_f"},                   # 試作: cpr で使わないときは既定の fallback に任せる (道筋が既定と同じ)
    "cpf2": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe_f", "ENOMOTO_T_MIP_CLIQUE_PROBE_TIME_FRAC": "0.02"},  # 試作: cpf で比べの時間を全体の 2% まで
    "cp3": {"ENOMOTO_MIP_CLIQUE_CUTS": "probe", "ENOMOTO_T_MIP_CLIQUE_PROBE_ROUNDS": "3"},
    "cq": {"ENOMOTO_MIP_CLIQUE_CUTS": "1"},                         # 常用 (根とノード)
    "cr": {"ENOMOTO_MIP_CLIQUE_CUTS": "root"},                      # 常用 (根だけ)
}
# クリークの行の多い問題 (HILOSS で fallback が却下・害のあった問題と、drayage)
CQ = ["rococoC10-001000", "neos-1456979", "neos859080", "hypothyroid-k1", "30n20b8", "net12", "drayage-25-23", "drayage-100-23"]
SETS = {"@58": P58, "@cq": CQ}
out, cfgs, seeds, tl = Path(sys.argv[1]), sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")], int(sys.argv[4])
probs = []
for a in sys.argv[5:]:
    if a.startswith("@k"):
        i, n = (int(v) for v in a[2:].split("/"))
        probs += [p for j, p in enumerate(P58) if j % n == i]
    else:
        probs += SETS.get(a, [a])
if not out.is_absolute():
    out = D / out
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
logdir = os.environ.get("CF_LOGDIR")
for seed in seeds:
    cs = cfgs if seed % 2 else cfgs[::-1]
    for p in probs:
        for c in cs:
            if (p, c, seed) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if os.environ.get("CF_PKG"):
                env["PYTHONPATH"] = os.environ["CF_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
            for kv in filter(None, os.environ.get("CF_ENV", "").split(",")):
                k_, v_ = kv.split("=", 1); env[k_] = v_
            if os.environ.get("CF_LOG"):
                env["ENOMOTO_MIP_LOG"] = "1"
            mps = WT / ".miplib_cache" / f"{p}.mps"
            if not mps.exists():
                sys.path.insert(0, str(WT / "scripts"))
                import run_miplib_benchmark as rb  # noqa: E402
                rb.fetch(p, WT / ".miplib_cache")
            t0 = time.time()
            cl = []
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                cl = [l for l in r.stderr.splitlines() if "clique probe" in l or "clique fallback" in l]
                if logdir:
                    Path(logdir).mkdir(parents=True, exist_ok=True)
                    (Path(logdir) / f"{p}.{c}.s{seed}.log.gz").write_bytes(gzip.compress(r.stderr.encode()))
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": tl, "result": res, "clique": cl,
                                    "wall": round(time.time() - t0, 1), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), res.get("root_bound"), cl[-1:], flush=True)
