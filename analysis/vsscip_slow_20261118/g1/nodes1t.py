"""既定化の確かめ: 1 スレッドで数問を解き、ノード数・目的値・限界を JSON 行で出す (2 つの版で同じになることを見る)。
python nodes1t.py タグ [環境変数=値 ...] >> nodes1t.jsonl (リポジトリの根から。ノード数の上限 ENOMOTO_MIP_NODE_LIMIT=5000、時間 600 秒)"""
import json, os, subprocess, sys
PROBS = ["graph20-20-1rand", "neos-787933", "neos8", "misc07", "neos-911970", "p0201", "physiciansched6-2", "rout"]
tag = sys.argv[1]
env = dict(os.environ, ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_NODE_LIMIT="5000")
env.update(kv.split("=", 1) for kv in sys.argv[2:])
for p in PROBS:
    out = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps",
                          "--time-limit", "600", "--threads", "1"], env=env, capture_output=True, text=True).stdout
    r = json.loads(out.strip().splitlines()[-1])
    print(json.dumps({"tag": tag, "problem": p, "env": sys.argv[2:], "status": r["status"], "nodes": r["nodes"],
                      "objective": r["objective"], "bound": r["bound"], "time": round(r["time"], 2)}), flush=True)
