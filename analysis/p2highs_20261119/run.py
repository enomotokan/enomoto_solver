"""P2-HIGHS (HiGHS だけ解ける問題) の調べ: python run.py 問題:設定 [...] → res.jsonl と logs/問題_設定.log.gz。
1 スレッド・600 秒・種 1、打ち切り値なし (普通の求解)。当方は ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_TREE_LOG=1 (ノードの行 ENODE・EBR)、
HiGHS は計装つき 1.15.1 (環境変数 HIGHSPY_ENO にビルドの場所、treeclose2_20261117 の README) で HIGHS_TREE_LOG=1 (HNODE・HBR)。
同じ (問題, 設定) が res.jsonl にあれば飛ばす (途中から再開できる)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402

# 設定: 名前 → (HiGHS か, 環境変数)
CFGS = {
    "highs_t1": (True, {"HIGHS_TREE_LOG": "1"}),
    "ours_t1": (False, {"ENOMOTO_MIP_TREE_LOG": "1"}),
    # 前処理の畳み込み (MIP_FOLD) を切る / さらに再スタート (根・木の中) も切る: HiGHS とモデルをそろえる比べ
    "ours_t1_nofold": (False, {"ENOMOTO_MIP_TREE_LOG": "1", "ENOMOTO_MIP_FOLD": "0"}),
    "ours_t1_nofold_nors": (False, {"ENOMOTO_MIP_TREE_LOG": "1", "ENOMOTO_MIP_FOLD": "0", "ENOMOTO_MIP_NO_RESTART": "1"}),
}
OUT = D / "res.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(OUT)} if OUT.exists() else set()
for name in sys.argv[1:]:
    p, c = name.split(":")
    if (p, c) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    hi, extra = CFGS[c]
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(extra)
    if hi:
        env["PYTHONPATH"] = os.environ["HIGHSPY_ENO"]
        env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
    else:
        env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs" if hi else "--worker-ours", str(mps),
                        "--time-limit", "600", "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=1500)
    (LOGS / f"{p}_{c}.log.gz").write_bytes(gzip.compress((r.stdout + r.stderr).encode()))
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
