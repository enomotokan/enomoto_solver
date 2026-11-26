"""研磨の作り直し (1 = ROOTPAR_PH の i、1+2) の比べ: 1 問を 1 つの形で解き、結果を jsonl に足し、ログ (研磨の統計つき) を残す (2026-11-26)。
使い方: python polcmp.py <出力 jsonl> <問題> <スレッド> <秒> <名前> [K=V ...]
ログは polcmp/log_<問題>_<名前>_t<スレッド>_<秒>.txt、最後の暫定解は polcmp/x_….json (nattai の fixlp.py の確かめ用)。"""
import json, os, sys, time
from pathlib import Path
R = Path(__file__).resolve().parents[2]
out, prob, th, tl, name = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4]), sys.argv[5]
env = dict(kv.split("=", 1) for kv in sys.argv[6:])
env.setdefault("ENOMOTO_MIP_POLISH_STATS", "v")
os.environ.update(env)
from enomoto_solver import benchmark_highs as bh
m, _, _, names = bh._read_our_model(R / f".miplib_cache/{prob}.mps")
D = Path(__file__).parent / "polcmp"; D.mkdir(exist_ok=True)
tag = f"{prob}_{name}_t{th}_{int(tl)}"
fd = os.open(D / f"log_{tag}.txt", os.O_WRONLY | os.O_CREAT | os.O_TRUNC); os.dup2(fd, 2)
t = time.perf_counter()
r = m.solve(None, False, tl, None, None, th, True, None)
t = time.perf_counter() - t
x = dict(zip(names, list(r["x"]))) if r.get("x") is not None else None
(D / f"x_{tag}.json").write_text(json.dumps({"objective": r["objective"], "x": x}))
row = {"prob": prob, "th": th, "tl": tl, "name": name, "env": env, "status": r["status"], "objective": r["objective"],
       "bound": r["best_bound"], "nodes": r["nodes"], "time": t}
with open(out, "a") as f: f.write(json.dumps(row) + "\n")
print(json.dumps(row))
