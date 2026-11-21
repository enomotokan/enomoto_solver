"""既定にした fit・r3e・rck の確かめ (親の依頼、2026-11-21): 変更前のビルド (old) + 3 つの環境変数 と、新しい既定のビルド (new) + 環境変数なし で、
1 スレッドの同じ問題のノード・目的値・下界・試作の行 (CUTSEL_PROBE_FIT・ROOTLP3・ROOTCAP) が一致するか。
python eq.py OLD.whl NEW.whl (リポジトリの外の wheel)。old を全部回してから new を回す (入れ直しは 2 回)。ログは logs/<問題>_<old|new>.log、結果は res.jsonl。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
PROBS = [("10teams", 60), ("mas76", 60), ("noswot", 60), ("timtab1", 60), ("rail01", 120), ("neos-933966", 300), ("sorrell3", 300)]
# 時間切れでノードが時刻で決まる 2 問は、ノードの上限 (ENOMOTO_MIP_NODE_LIMIT) で止めた回も比べる (名前は <問題>@<上限>)
PROBS += [("10teams@5000", 60), ("neos-933966@500", 300)]
ENV3 = {"ENOMOTO_MIP_CUTSEL_PROBE_FIT": "1", "ENOMOTO_MIP_ROOTLP3": "e", "ENOMOTO_MIP_ROOTCAP": "k"}
OUT = D / "res.jsonl"
done = {(r["problem"], r["build"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
PAT = re.compile(r"CUTSEL_PROBE_FIT|ROOTLP3|ROOTCAP")
for build, whl in (("old", sys.argv[1]), ("new", sys.argv[2])):
    if all((p, build) in done for p, _ in PROBS):
        continue
    subprocess.run([sys.executable, "-m", "pip", "install", "-q", "--force-reinstall", "--no-deps", whl], check=True)
    for prob, tl in PROBS:
        if (prob, build) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        if build == "old":
            env.update(ENV3)
        env.update(ENOMOTO_MIP_THREADS="1", ENOMOTO_MIP_LOG="1")
        if "@" in prob:
            env["ENOMOTO_MIP_NODE_LIMIT"] = prob.split("@")[1]
        log = D / "logs" / f"{prob.replace('@', '_nl')}_{build}.log"
        with open(log, "w") as fe:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{prob.split('@')[0]}.mps"),
                                 "--time-limit", str(tl), "--threads", "1"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        lines = [re.sub(r"[\d.]+s\b", "Xs", l.strip()) for l in open(log) if PAT.search(l)]
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": prob, "build": build, "result": res, "proto_lines": lines, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(prob, build, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), res.get("time"), len(lines), flush=True)
