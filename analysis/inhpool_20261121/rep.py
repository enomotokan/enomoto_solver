"""noswot・10teams の切り分け (README の「noswot の切り分け」節): python rep.py → res_rep.jsonl。
追加: python rep.py 問題 種の始め 種の終わり [t1] で、その問題の種を足す (t1 なら old_t1・def_t1 の 1 スレッドで、各 1 回)。
noswot は種 1〜10、10teams は種 1〜6、それぞれ {old, def} × 2 回 (4 スレッド・60 秒)。run.py と同じ worker・行の形に "rep" を足す。
同じ (問題, 種, 回) の old と def を続けて回し、2 回目は順を入れ替える (機械の揺れが片方に寄らないように)。途中から再開できる。"""
import json, os, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv_orig = list(sys.argv)
sys.argv = [sys.argv[0], "P", "0"]  # run.py の読み込みで分担表を読ませるための形だけ (回すのは下の並び)
src = (D / "run.py").read_text().split("for p, cfg, seed, tl in mine:")[0]
exec(compile(src, str(D / "run.py"), "exec"))
OUT = D / "res_rep.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"], r["rep"]))
CFGS["old_t1"] = CFGS["old"]; CFGS["def_t1"] = CFGS["def"]
ARGS = sys.argv_orig[1:]
plan = []
T1 = len(ARGS) > 3 and ARGS[3] == "t1"
probs = ((ARGS[0], range(int(ARGS[1]), int(ARGS[2]) + 1)),) if ARGS else (("noswot", range(1, 11)), ("10teams", range(1, 7)))
for p, seeds in probs:
    for s in seeds:
        for rep in ((1,) if T1 else (1, 2)):
            for cfg in (("old", "def") if rep == 1 else ("def", "old")):
                plan.append((p, cfg + ("_t1" if T1 else ""), s, rep))
for p, cfg, seed, rep in plan:
    if (p, cfg, seed, rep) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    res, peak, wall = run(cfg, seed, mps, 60)
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "rep": rep, "tl": 60, "result": res, "max_rss_mb": peak, "wall": wall,
                            "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, cfg, seed, rep, res.get("status"), res.get("time"), res.get("nodes"), flush=True)
