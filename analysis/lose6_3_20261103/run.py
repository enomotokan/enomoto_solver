"""lose6_3 (2026-11-03): HiGHS に負けた 6 問のうち drayage-25-23・p200x1188c を ENOMOTO (4 スレッド・60 秒) と HiGHS で解く。
ENOMOTO は ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1・ENOMOTO_DEBUG_RACE=1 (どれも表示だけ)。標準エラーの各行に起動からの経過秒を付ける。
使い方: python run.py <問題,...> <設定,...> <回数> [初めの回]
  設定: highs (--worker-highs、時間だけ)・highslog (同じ設定でログを出す)・def (今の既定)・ほかは CFGS (環境変数)。
  ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl。ソルバーは同時に 1 つ。設定は回ごとに交互。"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
HIGHSLOG = r'''
import sys, time, json, highspy
h = highspy.Highs(); h.setOptionValue("time_limit", 60.0); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); t = time.perf_counter() - t; i = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value, "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": t}))
'''
CFGS = {"def": {}}
for k, v in json.loads(os.environ.get("L3_EXTRA_CFGS", "{}")).items():
    CFGS[k] = v
CFGS.update({
    # 試作 (src/mip/solver.rs、ENOMOTO_MIP_LOSE6_3_*)
})


def run_one(P, mps, c, i):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    if c == "highs":
        cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "60", "--threads", "4"]
    elif c == "highslog":
        cmd = [sys.executable, "-c", HIGHSLOG, str(mps)]
    else:
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", ENOMOTO_DEBUG_RACE="1", **CFGS[c])
        cmd = [sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"]
    t0 = time.perf_counter()
    pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1)
    errs = []
    th = threading.Thread(target=lambda: [errs.append(f"[{time.perf_counter() - t0:7.3f}] {l}") for l in pr.stderr])
    th.start(); out = pr.stdout.read(); pr.wait(timeout=300); th.join()
    (D / "logs" / f"{P}_{c}_{i}.log").write_text("".join(errs) + out)
    js = [l for l in out.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {"status": "crash", "time": -1}
    res.update(problem=P, cfg=c, run=i, env=CFGS.get(c, {}))
    with open(D / "res.jsonl", "a") as f:
        f.write(json.dumps(res) + "\n")
    print(f"{P:16s} {c:10s} {i} {res['status']:10s} {res['time']:6.2f}s obj={res.get('objective')} rb={res.get('root_bound')} nodes={res.get('nodes')}", flush=True)
    return res


if __name__ == "__main__":
    probs, cfgs, n = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    i0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    for P in probs:
        mps = rb.fetch(P, WT / ".miplib_cache")
        for i in range(i0, i0 + n):
            for c in cfgs:
                run_one(P, mps, c, i)
