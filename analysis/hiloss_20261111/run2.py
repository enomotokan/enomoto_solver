"""HILOSS の切り分けの実験: 負けている問題を、既存の試作の環境変数を 1 つずつ足して 600 秒・4 スレッド・種 1 で回す (既定は変えない)。
使い方: python run2.py K [問題 設定]   (shards2.json の K 番目の分担。途中から再開できる。ソルバーは同時に 1 つ)
        HILOSS_SET=3 python run2.py K   (shards3.json の分担、出力は res3_sK.jsonl)
出力: res2_sK.jsonl (1 行 = 問題 x 設定)、logs2/<問題>.<設定>.log.gz (各行の頭にプロセスの開始からの経過秒)。
どの回も ENOMOTO_MIP_LOG=1 と ENOMOTO_MIP_TREE4_TRACE=10 (ログだけ)。
前もって: pip install maturin highspy && pip install -e . (リポジトリの根で)"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "cq": {"ENOMOTO_MIP_CLIQUE_CUTS": "1"},                                        # クリークカットを根のループで常に使う (既定は fallback)
    "cr": {"ENOMOTO_T_MIP_CUT_ROUNDS": "100"},                                     # 根のカットのラウンドの上限 25 -> 100
    "cqr": {"ENOMOTO_MIP_CLIQUE_CUTS": "1", "ENOMOTO_T_MIP_CUT_ROUNDS": "100"},
    "sj": {"ENOMOTO_MIP_SUBCAP": "JB"},                                            # 根の並列のサブ MIP を待つ時間に上限 (analysis/submip_cap_20261106.md の改良)
    "ru": {"ENOMOTO_MIP_TREE4_RCUNION": "1"},                                      # 根のカットの各ラウンドの LP の被約費用でも固定する
    "sd": {"ENOMOTO_MIP_SB_NO_DEGEN_SKIP": "1"},                                   # 双対退化の強いノードでも強分岐を止めない
    "t1": {"ENOMOTO_MIP_TREE1_SB": "1"},                                           # HiGHS の信頼性分枝にそろえる (予算 100000 + 0.5 x、閾値 8)
}
TL = 600
k = int(sys.argv[1])
SET = os.environ.get("HILOSS_SET", "2")
mine = json.loads((D / f"shards{SET}.json").read_text())["shards"][k]
if len(sys.argv) > 3: mine = [[sys.argv[2], sys.argv[3]]]
OUT = D / f"res{SET}_s{k}.jsonl"
(D / "logs2").mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"]))
def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0
def run(cfg, mps, log):
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TREE4_TRACE="10")
    t0 = time.time(); peak = 0.0; last = [""]; buf = []
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    def pump():
        for line in pr.stdout:
            if line.startswith("{"): last[0] = line
            buf.append(f"{time.time() - t0:8.2f} {line}")
    th = threading.Thread(target=pump); th.start()
    while pr.poll() is None:
        peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
        if time.time() - t0 > TL + 300: pr.kill(); break
    th.join()
    log.write_bytes(gzip.compress("".join(buf).encode()))  # 終わってからまとめて書く (途中のコミットに空のファイルが入らないように)
    try: res = json.loads(last[0])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(peak), round(time.time() - t0, 1)
for p, cfg in mine:
    if (p, cfg) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    res, peak, wall = run(cfg, mps, D / "logs2" / f"{p}.{cfg}.log.gz")
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": 1, "tl": TL, "result": res, "max_rss_mb": peak, "wall": wall, "shard": k,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, res.get("status"), res.get("time"), peak, flush=True)
