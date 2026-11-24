"""試作 ENOMOTO_SIMPLEX_STABLE_TIE の確かめ: LU_EXTEND を根でも使う形 (`=r`) と既定の分解で、根のカットのループの各周 (カットの数・LP の反復・目的値)
と単体法の反復ごとの決定 (出る行・入る列・ピボット・ステップ・フリップの数) が一致するかを比べる (1 スレッド・種 1)。
使い方 (リポジトリの根から): python analysis/luxroot_20261124/stable.py <秒>   出力: res_stable.jsonl、ログ logs/st_<設定>_<問題>.log.gz、比べた決定の列 logs/st_seg_<設定>_<問題>.txt.gz"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "120"
probs = ["nw04", "neos-827175", "air03"]
cfgs = [("def", {}), ("extr", {"ENOMOTO_MIP_LU_EXTEND": "r"}), ("st", {"ENOMOTO_SIMPLEX_STABLE_TIE": "1"}),
        ("st_extr", {"ENOMOTO_SIMPLEX_STABLE_TIE": "1", "ENOMOTO_MIP_LU_EXTEND": "r"}),
        ("sth", {"ENOMOTO_SIMPLEX_STABLE_TIE": "h"}), ("sth_extr", {"ENOMOTO_SIMPLEX_STABLE_TIE": "h", "ENOMOTO_MIP_LU_EXTEND": "r"})]
out = D / "res_stable.jsonl"
done = set()
if out.exists():
    for l in open(out):
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
def root_part(log):
    # 最初の根の LP からカットのループの終わりまでの、反復の決定と周の行 (丸めの雑音の値 dj・xr・w_r と時間は除く)
    seg, on = [], False
    for l in log.splitlines():
        if "solving the root LP" in l:
            on = True
        if not on:
            continue
        if l.startswith("ITERLOG") or re.match(r"MIP: cut round \d+: \d+ cuts", l) or l.startswith("MIP: cut loop done"):
            l = re.sub(r" (dj|xr|w_r)=\S+", "", l); l = re.sub(r"\([\d.]+s\)|sep [\d.]+s, ", "", l)
            seg.append(l.strip())
        if l.startswith("MIP: cut loop done"):
            break
    return seg
for p in probs:
    for cname, cenv in cfgs:
        if (p, cname) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(cenv)
        env.update({"ENOMOTO_DEBUG_EXT_ITERLOG": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
        r = subprocess.run([sys.executable, "scripts/run_miplib_benchmark.py", "--worker-ours", f".miplib_cache/{p}.mps", "--time-limit", tl, "--threads", "1"],
                           cwd=R, env=env, capture_output=True, text=True)
        log = r.stdout + r.stderr
        with gzip.open(D / "logs" / f"st_{cname}_{p}.log.gz", "wt") as f:
            f.write(log)
        res = next((json.loads(l) for l in reversed(log.splitlines()) if l.startswith("{")), None)
        seg = root_part(log)
        rounds = [l for l in seg if l.startswith("MIP: cut round")]
        with gzip.open(D / "logs" / f"st_seg_{cname}_{p}.txt.gz", "wt") as f:
            f.write("\n".join(seg) + "\n")
        row = {"problem": p, "cfg": cname, "result": res, "rounds": rounds, "n_iterlog": sum(l.startswith("ITERLOG") for l in seg)}
        open(out, "a").write(json.dumps(row) + "\n")
        rr = res or {}
        print(p, cname, rr.get("status"), rr.get("nodes"), "%.1f" % (rr.get("time") or 0), "rounds", len(rounds), flush=True)
# 比べ: 既定の分解 (def / st) と根でも延ばす (extr / st_extr) の組で、最初に分かれる行
rows = {}
for l in open(out):
    r = json.loads(l); rows[(r["problem"], r["cfg"])] = r
for p in probs:
    for a, b in (("def", "extr"), ("st", "st_extr"), ("sth", "sth_extr")):
        if (p, a) not in rows or (p, b) not in rows:
            continue
        sa, sb = (gzip.open(D / "logs" / f"st_seg_{c}_{p}.txt.gz", "rt").read().splitlines() for c in (a, b))
        k = next((i for i in range(min(len(sa), len(sb))) if sa[i] != sb[i]), None)
        if k is None and len(sa) == len(sb):
            print(f"{p} {a} vs {b}: 根のカットのループまで全部一致 ({len(sa)} 行)")
        else:
            k = k if k is not None else min(len(sa), len(sb))
            nr = sum(l.startswith("MIP: cut round") for l in sa[:k])
            print(f"{p} {a} vs {b}: {k} 行目で分かれる (それまでに周 {nr} 個): {a}: {sa[k] if k < len(sa) else '-'} | {b}: {sb[k] if k < len(sb) else '-'}")
