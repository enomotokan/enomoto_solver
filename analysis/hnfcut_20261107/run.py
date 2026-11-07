"""既定 (def)・HNFPRE=1 (hnf)・HNFCUT (hcut = abc、hcut_a・hcut_b・hcut_c は文字ごと) を 4 スレッドで解く (2026-11-07)。1 回ずつ順に。
使い方: python run.py out.jsonl T cfgs(カンマ区切り) seeds(カンマ区切り) 問題...  ("@cong" で合同式の出る問題、"@p58" で 58 問)。
既に out にある (問題, 設定, 回) は飛ばす (止まったら同じ命令で続きから回る)。根の LP・根のカットの後の下界・HNFCUT の数え上げをログから取る。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
import hprobs
CFG = {"def": {}, "hnf": {"ENOMOTO_MIP_HNFPRE": "1"}, "hcut": {"ENOMOTO_MIP_HNFCUT": "1"},
       "hcut_a": {"ENOMOTO_MIP_HNFCUT": "a"}, "hcut_b": {"ENOMOTO_MIP_HNFCUT": "b"}, "hcut_c": {"ENOMOTO_MIP_HNFCUT": "c"}, "hcut_ac": {"ENOMOTO_MIP_HNFCUT": "ac"}}
out = Path(sys.argv[1]); T = float(sys.argv[2]); cfgs = sys.argv[3].split(","); seeds = [int(s) for s in sys.argv[4].split(",")]
probs = []
for a in sys.argv[5:]:
    probs += hprobs.SETS[a[1:]] if a.startswith("@") else [a]
done = set()
if out.exists():
    for l in out.open():
        try:
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
        except Exception:
            pass
F = r"([-+0-9.eE]+|inf|-inf|NaN)"
for s in seeds:
    for p in probs:
        for c in cfgs:
            if (p, c, s) in done:
                continue
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s), **CFG[c])
            for k in ("ENOMOTO_MIP_HNFPRE", "ENOMOTO_MIP_HNFCUT"):
                if k not in CFG[c]:
                    env.pop(k, None)
            mps = WT / ".miplib_cache" / f"{p}.mps"
            t0 = time.time()
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"],
                                    capture_output=True, text=True, env=env, timeout=T * 3 + 120)
                so, se = pr.stdout, pr.stderr
            except subprocess.TimeoutExpired as e:
                so, se = "", (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
            try:
                res = json.loads(so.strip().splitlines()[-1])
            except Exception:
                res = {"status": "error", "err": se[-500:]}
            info = {}
            m = re.search(r"MIP: root LP " + F, se)
            if m: info["root_lp"] = float(m.group(1))
            m = re.search(r"MIP: cut loop done: (\d+) cuts added.*obj " + F + r" -> " + F, se)
            if m: info["cuts_added"] = int(m.group(1)); info["root_after_cuts"] = float(m.group(3))
            m = re.search(r"MIP: root (HNFCUT stats: .*)", se)
            if m: info["root_stats"] = m.group(1)
            ms = re.findall(r"^(HNFCUT stats: .*)$", se, re.M)
            if ms: info["stats"] = ms[-1]
            m = re.search(r"HNFPRE: final table: (\d+) congruences", se)
            if m: info["congs"] = int(m.group(1))
            rec = {"problem": p, "cfg": c, "rep": s, "result": res, "info": info, "wall": round(time.time() - t0, 2)}
            with out.open("a") as f:
                f.write(json.dumps(rec) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("nodes"), res.get("root_bound"), info.get("root_after_cuts"), flush=True)
