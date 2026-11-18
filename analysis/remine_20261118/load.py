import os
HERE = os.path.dirname(os.path.abspath(__file__)) + "/"
DIR = os.environ.get("REMINE_TMP", "/tmp/remine_20261118/")  # 中間の pickle (リポジトリに入れない)
os.makedirs(DIR, exist_ok=True)
import re, json, glob, os, collections, pickle
D = collections.defaultdict(dict)  # dir -> {(p,cfg,seed): (t, ok, tl, threads)}
for f in glob.glob("analysis/**/res*.jsonl", recursive=True):
    d0 = os.path.dirname(f)
    for l in open(f, errors="replace"):
        try: d = json.loads(l)
        except: continue
        if not isinstance(d, dict) or "problem" not in d or "result" not in d: continue
        cfg = d.get("cfg") or d.get("env") or d.get("solver")
        if isinstance(cfg, dict): cfg = json.dumps(cfg, sort_keys=True)
        if cfg is None: continue
        r = d["result"]
        if not isinstance(r, dict): continue
        tl = d.get("tl") or 60
        s = str(r.get("status", "")).lower()
        if s in ("crash", "fetch_error", "error"): continue
        t = r.get("time") or r.get("wall") or d.get("wall") or tl
        try: t = float(t)
        except: t = tl
        ok = s == "optimal" or s == "infeasible"
        t = min(t, tl) if ok else tl
        seed = d.get("seed", d.get("rep", 0))
        th = d.get("threads") or (1 if re.search(r"_t1(_|\.)", os.path.basename(f)) or str(cfg).endswith("_t1") else 4)
        if str(cfg).endswith("_t1") and cfg != "def_t1": pass
        key0 = d0 + ("#t1" if th == 1 else "")
        pn = d["problem"] if float(tl) == 60 else f"{d['problem']}@{int(float(tl))}"  # 上限の違う回 (P の 60 秒と M の 600 秒) を混ぜない
        D[key0][(pn, str(cfg), seed)] = (t, ok, tl, th, f)
pickle.dump(dict(D), open(DIR+"D.pkl", "wb"))
n = 0
for d0, R in sorted(D.items()):
    C = collections.Counter(k[1] for k in R)
    S = sorted({str(k[2]) for k in R})
    n += 1
    print(d0, len(R), "seeds", ",".join(S)[:40], "cfgs", " ".join(f"{c}:{v}" for c, v in sorted(C.items()))[:300])
