"""HEURGAP (ヒューリスティクスの弱点、2026-11-17。analysis/primal_20261117/run.py の写し): python run.py 問題,... [種] [highs|ours|both]
HiGHS 1.15.1 (H115 に highspy 1.15.1 の venv の python を渡す) と当方 (既定、ENOMOTO_MIP_LOG=1) を 4 スレッド・600 秒で 1 問ずつ順に解き
(同時に 1 つ)、ログを logs/highs_<問題>_s<種>.log・logs/ours_<問題>_s<種>.log に、結果の 1 行を res.jsonl に足す。
HiGHS の設定は scripts/run_miplib_benchmark.py の --worker-highs と同じ (threads 4・time_limit 600・mip_rel_gap 0、random_seed = 種)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
# 環境変数 TAG (ログ・結果の札) と OURS_ENV ("名=値;名=値"、当方に足す環境変数) で試作を回す
TAG = os.environ.get("TAG", ""); XENV = dict(kv.split("=", 1) for kv in os.environ.get("OURS_ENV", "").split(";") if kv)
TL = os.environ.get("TL", "600")  # 時間の上限 [秒] (当方だけ)
probs = sys.argv[1].split(","); seed = int(sys.argv[2]) if len(sys.argv) > 2 else 1; which = sys.argv[3] if len(sys.argv) > 3 else "both"
H115 = os.environ.get("H115", sys.executable)
HCODE = """
import json, sys, time, highspy
h = highspy.Highs()
h.setOptionValue("output_flag", True); h.setOptionValue("log_to_console", False); h.setOptionValue("log_file", sys.argv[2])
h.setOptionValue("threads", 4); h.setOptionValue("time_limit", 600.0); h.setOptionValue("random_seed", int(sys.argv[3])); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t
i = h.getInfo()
print("RES " + json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t, "version": highspy.Highs().version()}))
"""
for p in probs:
    mps = WT / ".miplib_cache" / f"{p}.mps"
    for who in (["highs", "ours"] if which == "both" else [which]):
        lf = D / "logs" / f"{who}{TAG}_{p}_s{seed}.log"
        if who == "highs":
            pr = subprocess.run([H115, "-c", HCODE, str(mps), str(lf), str(seed)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            res = [l for l in pr.stdout.splitlines() if l.startswith("RES ")]
            r = json.loads(res[-1][4:]) if res else {"status": "crash"}
        else:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"}); env.update(XENV)
            # 当方のログの時刻は再スタートごとに 0 に戻るので、標準エラーの各行の頭に起動からの実時間 [秒] を付ける
            t0 = time.time()
            pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                  cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
            out = []
            th = __import__("threading").Thread(target=lambda: out.append(pr.stdout.read())); th.start()
            with open(lf, "w") as f:
                for line in pr.stderr:
                    f.write(f"[{time.time() - t0:8.2f}] {line}")
            pr.wait(); th.join()
            try: r = json.loads([l for l in out[0].splitlines() if l.startswith("{")][-1])
            except Exception: r = {"status": "crash"}  # noqa: BLE001
        row = {"problem": p, "who": who + TAG, "env": XENV if who == "ours" else {}, "tl": TL if who == "ours" else "600", "seed": seed, "at": time.strftime("%H:%M:%S")} | {k: r.get(k) for k in ("status", "objective", "bound", "best_bound", "nodes", "time", "version")}
        with open(D / "res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)
