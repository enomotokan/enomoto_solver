"""カットの打ち切り (ENOMOTO_MIP_CUT_INTOBJ_STALL、待つ巡数 ENOMOTO_T_MIP_CUT_INTOBJ_STALL 既定 3) が外側の根で働く問題を数える:
python scan_st.py 部分 部分の数 → scan_st_部分.jsonl。Mittelmann 186 問を部分に分け、既定・1 スレッド・種 1 で解き、根のカットのループの終わり
("cut loop done") まで見て止める (180 秒まで)。打ち切りの行 "LP bound unchanged for k rounds; stop" が出たか、ループの巡数と時刻を記録する。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
MP = sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
k, nk = int(sys.argv[1]), int(sys.argv[2])
OUT = D / f"scan_st_{k}.jsonl"
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
for p in MP[k::nk]:
    if p in done: continue
    rec = {"problem": p, "stall": None, "rounds": None, "t_end": None, "err": None}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "180", "--threads", "1"],
                              cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        t0 = time.time(); rounds = 0
        for line in pr.stderr:
            if re.match(r"MIP: cut round \d+:", line) and "cuts (of" in line:
                rounds += 1
            if "LP bound unchanged for" in line and "stop" in line:
                rec["stall"] = line.strip()[:120]
            if line.startswith("MIP: cut loop done"):
                rec["t_end"] = round(time.time() - t0, 1)
                break
            if time.time() - t0 > 240:
                break
        rec["rounds"] = rounds
        pr.kill(); pr.wait()
    except Exception as e:  # noqa: BLE001
        rec["err"] = str(e)[:100]
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(p, rec["stall"] is not None, rec["rounds"], rec["t_end"], flush=True)
