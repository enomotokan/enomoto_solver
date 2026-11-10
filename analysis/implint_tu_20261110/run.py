"""PAPER7 の計測: 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足す (途中から再開できる。設定の順は種ごとに逆)。
種は ENOMOTO_T_MIP_SEED=種。計測する wheel は環境変数 IMPLINT_PKG (pip install --target の場所) を PYTHONPATH の先に置いて固定する
(計測の途中で作り直した wheel を拾わないため)。
使い方: python run.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...|@58|@47|@aff58|@aff47>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from probs_all import P58, P47  # noqa: E402
CFGS = {
    "def": {},
    "tu": {"ENOMOTO_MIP_IMPLINT_TU": "1"},
    "tu2": {"ENOMOTO_MIP_IMPLINT_TU2": "1"},
    "tu2a": {"ENOMOTO_MIP_IMPLINT_TU2": "ba"},
    "tu2as": {"ENOMOTO_MIP_IMPLINT_TU2": "bas"},
    "tu2ns": {"ENOMOTO_MIP_IMPLINT_TU2": "bns"},
    "defs": {"ENOMOTO_MIP_OBJ_STEP_IMPLINT": "1"},
    "tu2s": {"ENOMOTO_MIP_IMPLINT_TU2": "1", "ENOMOTO_MIP_OBJ_STEP_IMPLINT": "1"},
    "notu": {"ENOMOTO_MIP_NO_IMPLINT": "1"},
    "tu2ae": {"ENOMOTO_MIP_IMPLINT_TU2": "bae"},
    "tu2aes": {"ENOMOTO_MIP_IMPLINT_TU2": "baes"},
    "tu2aF": {"ENOMOTO_MIP_IMPLINT_TU2": "baF"},  # 診断 (誤答になりうる): TU と判定できない芯も TU とみなす
}
# TU2 で暗黙の整数の印が既定 (行の規則) から変わる問題 (count.jsonl)
AFF = [json.loads(l)["problem"] for l in open(D / "count.jsonl") if json.loads(l).get("tu2")]
SETS = {"@58": P58, "@47": P47, "@aff58": [p for p in P58 if p in AFF], "@aff47": [p for p in P47 if p in AFF]}
out, cfgs, seeds, tl = Path(sys.argv[1]), sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")], int(sys.argv[4])
probs = []
for a in sys.argv[5:]:
    probs += SETS.get(a, [a])
if not out.is_absolute():
    out = D / out
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for seed in seeds:
    cs = cfgs if seed % 2 else cfgs[::-1]
    for p in probs:
        for c in cs:
            if (p, c, seed) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if os.environ.get("IMPLINT_PKG"):
                env["PYTHONPATH"] = os.environ["IMPLINT_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **CFGS[c])
            t0 = time.time()
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": tl, "result": res, "wall": round(time.time() - t0, 1),
                                    "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), flush=True)
