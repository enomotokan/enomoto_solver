"""木を閉じる力の調べ (treeclose2、親の依頼 2026-11-17): python run.py 名前 [名前 ...] → res.jsonl と logs/問題_設定.log.gz。
名前は「問題:設定」(例 sct2:ours_cut)。600 秒・4 スレッド (設定に書いたものはその数)・種 1、最適値を打ち切り値に与える (整数の目的値は − 0.5、b59 の *_cut と同じ)。
設定 highs_* は計装を入れた HiGHS 1.15.1 (環境変数 HIGHSPY_ENO にそのビルドの場所。最後に ENOSTATS の行を出す) で解く。
同じ (問題, 設定) が res.jsonl にあれば飛ばす (途中から再開できる)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402

OPT = {"csched008": (173.0, True), "sct2": (-230.98916228571431, False), "neos-3627168-kasai": (988585.62, False)}
# 設定: 名前 → (HiGHS か, 環境変数)
CFGS = {
    "highs_cut": (True, {}),
    "highs_cut_nosym": (True, {"ENOMOTO_BENCH_HIGHS_OPT": "mip_detect_symmetry=false"}),
    "ours_cut": (False, {}),
    "ours_cut_nosym": (False, {"ENOMOTO_MIP_NO_SYMMETRY": "1"}),
    # リテラルの衝突を HiGHS のように長く持つ (年齢で消さない・枠 1 万。HiGHS は 1 万本を超えたときだけ年齢で消す)
    "ours_cut_keep": (False, {"ENOMOTO_T_MIP_LIT_CONFLICT_AGE": "4000000000", "ENOMOTO_T_MIP_LIT_CONFLICT_POOL": "10000",
                              "ENOMOTO_MIP_CONFLICT_LEV_DIAG": "1"}),
    # 診断だけ (衝突解析で深さごとに 1-UIP まで解けたか、解けずに残った変更の理由)
    "ours_cut_diag": (False, {"ENOMOTO_MIP_CONFLICT_LEV_DIAG": "1"}),
    # 試作: 双対証明・カットのプールの伝播で締めた変更も衝突解析で遡る
    "ours_cut_pe": (False, {"ENOMOTO_MIP_CONFLICT_PROOF_EXPLAIN": "1", "ENOMOTO_MIP_CONFLICT_LEV_DIAG": "1"}),
    # (2) ノードごとの記録 (1 スレッド = 木 1 本で並べる)。当方は ENODE・EBR、HiGHS は HNODE・HBR の行
    "highs_cut_t1_tl": (True, {"HIGHS_TREE_LOG": "1"}, "1"),
    "highs_cut_t1_norst": (True, {"HIGHS_TREE_LOG": "1", "ENOMOTO_BENCH_HIGHS_OPT": "mip_allow_restart=false"}, "1"),
    # 同じ形で、ノードの行に固まった整数列の数 (nfix) を足したビルドで取り直したもの
    "highs_cut_t1_norst_nf": (True, {"HIGHS_TREE_LOG": "1", "ENOMOTO_BENCH_HIGHS_OPT": "mip_allow_restart=false"}, "1"),
    "ours_cut_t1_tl_nf": (False, {"ENOMOTO_MIP_TREE_LOG": "1"}, "1"),
    # 潜る子の選び方: HiGHS の形 (推論の平均 / 擬似費用) と、常に上の子
    "ours_cut_t1_chh": (False, {"ENOMOTO_MIP_TREE_LOG": "1", "ENOMOTO_MIP_CHILD_HIGHS": "1"}, "1"),
    "ours_cut_t1_chup": (False, {"ENOMOTO_MIP_TREE_LOG": "1", "ENOMOTO_MIP_CHILD_HIGHS": "up"}, "1"),
    "ours_cut_t1_tl": (False, {"ENOMOTO_MIP_TREE_LOG": "1", "ENOMOTO_MIP_CONFLICT_LEV_DIAG": "1"}, "1"),
}
OUT = D / "res.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(OUT)} if OUT.exists() else set()
for name in sys.argv[1:]:
    p, c = name.split(":")
    if (p, c) in done: continue
    opt, integ = OPT[p]; cut = opt - 0.5 if integ else opt
    mps = rb.fetch(p, WT / ".miplib_cache")
    hi, extra, th = (CFGS[c] + ("4",))[:3]
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(extra)
    if hi:
        env["PYTHONPATH"] = os.environ["HIGHSPY_ENO"]
        env["ENOMOTO_BENCH_HIGHS_LOG"] = "1"
        env["ENOMOTO_BENCH_HIGHS_OPT"] = ",".join(filter(None, [env.get("ENOMOTO_BENCH_HIGHS_OPT"), f"objective_bound={cut!r}"]))
    else:
        env.update(ENOMOTO_MIP_THREADS=th, ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_CUTOFF=repr(cut))
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs" if hi else "--worker-ours", str(mps),
                        "--time-limit", "600", "--threads", th], cwd=WT, env=env, capture_output=True, text=True, timeout=1500)
    (LOGS / f"{p}_{c}.log.gz").write_bytes(gzip.compress((r.stdout + r.stderr).encode()))
    try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "cut": cut, "result": res, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, c, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
