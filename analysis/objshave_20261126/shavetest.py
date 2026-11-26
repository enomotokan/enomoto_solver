"""shaving の効きの見積り: 候補の問題に「目的 ≤ L」(L = combo2 第 2 段の既定 t4 の 600 秒後の下界) の行を足し、当方 (4 スレッド・種 1・120 秒) で
実行不能を示せるか、何秒・何ノードかを測る。最適値 > L なので答えは実行不能。示せれば、下界を L + gcd に上げられる。
既定は 600 秒でこの下界を超えられていない。python shavetest.py → shavetest.jsonl"""
import json, os, subprocess, sys, tempfile
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CANDS = ["chromaticindex1024-7", "neos-5052403-cygnet", "nexp-150-20-8-5", "supportcase10", "sorrell3", "neos-1354092", "glass-sc", "square41",
         "square47", "netdiversion", "lectsched-5-obj", "satellites2-40", "satellites2-60-fs", "seymour", "cvs16r128-89", "n3div36", "comp21-2idx", "net12"]
gap = {g["problem"]: g for g in json.loads((WT / "analysis/gap240_20261126/gap.json").read_text())}
OUT = D / "shavetest.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = int(os.environ.get("SHAVE_TL", "120"))
for p in CANDS:
    if p in done: continue
    L = gap[p]["def_t4"]["bound"]
    mps = rb.fetch(p, WT / ".miplib_cache")
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp()
    sense = h.getObjectiveSense()[1] if isinstance(h.getObjectiveSense(), tuple) else h.getObjectiveSense()
    idx = [j for j, c in enumerate(lp.col_cost_) if c != 0]; val = [lp.col_cost_[j] for j in idx]
    rhs = L - lp.offset_ + 1e-6
    if sense == highspy.ObjSense.kMinimize: h.addRow(-highspy.kHighsInf, rhs, len(idx), idx, val)
    else: h.addRow(rhs - 2e-6, highspy.kHighsInf, len(idx), idx, val)
    f = Path(tempfile.gettempdir()) / f"shave_{p}.mps"; h.writeModel(str(f))
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1"})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(f), "--time-limit", str(TL), "--threads", "4"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    f.unlink()
    row = {"problem": p, "L": L, "sense": str(sense), "res": res}
    with open(OUT, "a") as fo: fo.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
