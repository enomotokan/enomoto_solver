"""(analysis/treeclose_20261114.md 9.38 節、b58r/run_shard.py の写しで DFSBB=4 の d・L も stats に。ENOMOTO_MIP_DFSBB=4 のある版で回す) 分担 K を回す: shards_NAME.json の K 番目の (問題, 設定, 種) を 4 スレッド・制限時間で 1 回ずつ解き、NAME_sK.jsonl に足す。ログは logs/<問題>_<設定>_s<種>_<組>.log.gz に (組の名前を付けてほかの組と衝突しないようにする)、開いたノード・兄弟への戻り・基底の戻し・ノードの分離のラウンドは行の result.stats に。
使い方: python run_shard.py K NAME (b58s/ で、NAME は dl15)。途中から再開できる。ソルバーは同時に 1 つ。
設定は cfgs.py (f1・f3・f10・対照 2 つ)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sys.path.insert(0, str(D))
from cfgs import CFGS  # noqa: E402
k = int(sys.argv[1])
NAME = sys.argv[2] if len(sys.argv) > 2 else "gap1u"
plan = json.loads((D / f"shards_{NAME}.json").read_text())
TL = plan["tl"]
OUT = D / f"{NAME}_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
import re


def stats_of(err):
    """ログから: 最後の開いたノード (TCTRACE)、兄弟への戻り・基底の戻し (≒ 待ち行列から取り出し)、ノードの分離のラウンド (主のスレッド)"""
    st = {}
    tr = [l for l in err.splitlines() if l.startswith("TCTRACE")]
    if tr:
        m = re.search(r"nodes (\d+) open (\d+)", tr[-1])
        if m:
            st["nodes_main"], st["open_last"] = int(m.group(1)), int(m.group(2))
        st["open_max"] = max(int(x.group(1)) for x in (re.search(r"open (\d+)", l) for l in tr) if x)
    m = re.search(r"MIP: sibling backtracks (\d+); basis restores (\d+)", err)
    if m:
        st["sibling_backtracks"], st["basis_restores"] = int(m.group(1)), int(m.group(2))
    m = re.search(r"MIP: NODESEL_BANDIT: version (\d+), pulls \[(\d+), (\d+)\]", err)
    if m:
        st["nb_version"], st["nb_pulls"] = int(m.group(1)), [int(m.group(2)), int(m.group(3))]
    m = re.search(r"MIP: NODESEL_BANDIT5: intervals (\d+), GP best point PLUNGE_QUOT ([\d.]+) SIBLING_QUOT ([\d.]+) u ([\d.]+)", err)
    if m:
        st["nb5_intervals"], st["nb5_best"] = int(m.group(1)), [float(m.group(2)), float(m.group(3)), float(m.group(4))]
    m = re.search(r"MIP: NODESEL_BANDIT5 time: choose ([\d.]+)s", err)
    if m:
        st["nb5_choose_secs"] = float(m.group(1))
    m = re.search(r"skipped without incumbent (\d+)", err)
    if m:
        st["nb5_skipped"] = int(m.group(1))
    m = re.search(r"last proposal PLUNGE_QUOT ([\d.]+) SIBLING_QUOT ([\d.]+) u ([\d.]+)", err)
    if m:
        st["nb5_last"] = [float(m.group(1)), float(m.group(2)), float(m.group(3))]
    m = re.search(r"MIP: NODESEL_BANDIT5 hyp: last (\S+), refits (\d+), counts \(ell_p/ell_s/ell_u~noise:n\) ?(.*)", err)
    if m:
        st["nb5_hyp_last"], st["nb5_refits"] = m.group(1), int(m.group(2))
        st["nb5_hyp_counts"] = {t.rsplit(":", 1)[0]: int(t.rsplit(":", 1)[1]) for t in m.group(3).split()}
    m = re.search(r"MIP: DFSBB: (?:kind \d+, )?W (\d+) \(root LP iterations (\d+)\), plunges (\d+), ended by the work limit (\d+), ended at a leaf or by pruning (\d+)", err)
    if m:
        st["dfsbb_w"], st["root_iters"], st["dfsbb_plunges"], st["dfsbb_wcut"], st["dfsbb_leaf"] = (int(m.group(i)) for i in range(1, 6))
    m = re.search(r"MIP: DFSBB4: d ([\d.]+) \((\w+)\), L (\d+), root fractional (\d+), integers (\d+), mean depth of plunges ended at a leaf or by pruning ([\d.]+)", err)
    if m:
        st["dfsbb_d"], st["dfsbb_l"], st["root_frac"], st["n_int"], st["leaf_depth"] = float(m.group(1)), int(m.group(3)), int(m.group(4)), int(m.group(5)), float(m.group(6))
    m = re.search(r"MIP: adaptive node cuts: rounds (\d+) effective (\d+)", err)
    if m:
        st["node_cut_rounds"], st["node_cut_effective"] = int(m.group(1)), int(m.group(2))
    return st


for p, c, seed in plan["shards"][k]:
    if (p, c, seed) in done:
        continue
    for attempt in range(5):
        try:
            mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception as e:  # noqa: BLE001
            err = e; time.sleep(30)
    else:
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": {"status": "fetch_error", "error": str(err)[:200]}}) + "\n")
        continue
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_TCTRACE="1", **CFGS[c])
    t0 = time.time(); fold = []
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                            "--time-limit", str(TL), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 400)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        fold = [l for l in r.stderr.splitlines() if l.startswith("MIP_FOLD") or "TUBR" in l][:8]
        import gzip
        with gzip.open(D / "logs" / f"{p}_{c}_s{seed}_{NAME}.log.gz", "wt") as lf:
            lf.write(r.stderr)
        res["stats"] = stats_of(r.stderr)
    except Exception as e:  # noqa: BLE001
        res = {"status": "crash", "error": str(e)[:300]}
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "rep": seed - 1, "tl": TL, "result": res, "fold": fold,
                            "wall": round(time.time() - t0, 1), "shard": k, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), flush=True)
