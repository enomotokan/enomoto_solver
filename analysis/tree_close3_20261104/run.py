"""tree_close3 (2026-11-04): 担当 6 問などを ENOMOTO (設定ごとの環境変数) か HiGHS で解き、ログを logs/ に、結果を res.jsonl に足す。
使い方: python run.py SOLVER(ours|highs) CFG TL PROB [PROB ...]
CFG は def か、cfgs の鍵 (環境変数の組)。ログは ENOMOTO_MIP_LOG=1 / ENOMOTO_BENCH_HIGHS_LOG=1 の標準エラー。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = json.loads((D / "cfgs.json").read_text()) if (D / "cfgs.json").exists() else {}
solver, cfg, tl = sys.argv[1], sys.argv[2], int(sys.argv[3])
for p in sys.argv[4:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env["ENOMOTO_MIP_THREADS"] = "4"
    env["ENOMOTO_MIP_LOG"] = "1"; env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
    env.update(CFGS.get(cfg, {}))
    # 走っている間は追跡されない場所に書き、終わってから logs/ に移す (途中で git の rebase がファイルを作り直すと、書いている
    # ファイルが消えてログが失われていた)
    final = D / "logs" / f"{p}_{solver}_{cfg}_{tl}.log"
    log = Path(os.environ.get("TREE3_TMP", "/tmp")) / final.name
    t0 = time.time()
    with open(log, "w") as lf:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{solver}", str(mps),
                             "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
    if solver == "highs":  # HiGHS のログは標準出力に出る
        with open(log, "a") as lf:
            lf.write(pr.stdout)
    import shutil
    shutil.move(str(log), str(final))
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    with open(D / "res.jsonl", "a") as f:
        f.write(json.dumps({"problem": p, "solver": solver, "cfg": cfg, "tl": tl, "env": CFGS.get(cfg, {}), "wall": round(time.time() - t0, 1), "result": res}) + "\n")
    print(p, solver, cfg, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
