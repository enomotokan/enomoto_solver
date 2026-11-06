"""設定ごとに問題を 4 スレッドで解き、<出力>.jsonl に足す (済んだ組は飛ばす。設定の順は回ごとに逆)。
使い方: python bench.py 出力 時間 回 設定,設定 問題 ...   (問題が "58" なら analysis/cbcimpl_20261029/probs.py の 58 問)
設定: def (既定)、s/n/r/x/xr/f は ENOMOTO_MIP_PROBE_BASIS_GUARD の値。ENOMOTO_MIP_LOG=1 で、試作・根の LP の行だけを記録に残す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
out, tl, rep, cfgs = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4].split(",")
probs = sys.argv[5:]
if probs == ["58"]:
    sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
    from probs import PROBS as probs  # noqa: E402
if rep % 2 == 1:
    cfgs = cfgs[::-1]
OUT = D / f"{out}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
KEYS = ("probe basis", "root LP basis from", "LP probing: root LP", "the first LP stopped")
(D / "logs").mkdir(exist_ok=True)
for p in probs:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(base, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
        if c != "def":
            env["ENOMOTO_MIP_PROBE_BASIS_GUARD"] = c
        notes, rl = [], {}
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", tl, "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) + 600)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            err = r.stderr
            notes = [l[:300] for l in err.splitlines() if any(k in l for k in KEYS)][:8]
            # 根の LP (最初の主): 'MIP: root LP: <st>, <iters> iters ... (<t>s)' 形のログから拾う
            # 根の LP: 'MIP: root LP <目的値> (<反復> iters, <秒>s)'、根の終わり: 'MIP: tree search with N threads (<秒>s)'、
            # 根の下界: 裏のカットの 'obj A -> B' の B
            for l in err.splitlines():
                m = re.search(r"^MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", l)
                if m and "obj" not in rl:
                    rl.update(obj=m.group(1), it=int(m.group(2)), t=float(m.group(3)))
                m = re.search(r"^MIP: root LP stopped at the time limit \(([\d.]+)s", l)
                if m and "obj" not in rl:
                    rl.update(obj=None, stopped=float(m.group(1)))
                m = re.search(r"^MIP: tree search with \d+ threads \(([\d.]+)s\)", l)
                if m and "root_end" not in rl:
                    rl["root_end"] = float(m.group(1))
                m = re.search(r"^MIP: background cuts: \d+ rounds, .*obj \S+ -> (\S+) ", l)
                if m:
                    rl["root_bound"] = m.group(1)
            if float(tl) >= 300:
                (D / "logs" / f"{out}_{c}_{p}_{rep}.log").write_text(err)
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "rootlp": rl, "notes": notes, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), res.get("objective"), res.get("best_bound"), flush=True)
