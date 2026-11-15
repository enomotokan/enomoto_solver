"""runs.jsonl とログから表を作る。使い方: python summ.py [タグ ...]"""
import json, re, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent
tags = sys.argv[1:]
rows = [json.loads(l) for l in open(D / "runs.jsonl")]
by = {}
for r in rows:
    key = (r["tag"].rstrip("1234567890") if r["tag"].startswith("a") else r["tag"], r["threads"])
    if tags and key[0] not in tags:
        continue
    log = (D / r["log"]).read_text()
    rb = re.search(r'"root_bound": ([\d.]+)', log)
    ah = re.findall(r"after root heuristics: incumbent Some\(([\d.]+)\)", log)
    role = re.findall(r"HEURTRIG: tid \d+ start", log)
    t_role = re.search(r"FRAG: t ([\d.]+) tid \d+ ALNS .* role true", log)
    opt = [t for o, n, t, s in r["incumbents"] if o <= r["obj"] + 1e-6]
    t_opt = min(opt) if opt else None
    src = [s for o, n, t, s in r["incumbents"] if t == t_opt]
    lns = sum(float(x) for x in re.findall(r"FRAG: t [\d.]+ tid \d+ node .* secs ([\d.]+)", log))
    by.setdefault(key, []).append(r["time"])
    print(f"{key[0]:8s} t{r['threads']} s{r['seed']:<2} {r['time']:7.1f}s nodes {r['nodes']:5} root_inc {ah[-1] if ah else '-':>6} root_lb {float(rb.group(1)) if rb else 0:7.2f} "
          f"opt@{t_opt if t_opt is not None else 0:6.1f} [{src[0] if src else ''}] proof_after {r['time'] - (t_opt or 0):5.1f}s role_first {float(t_role.group(1)) if t_role else 0:5.1f} lns_secs {lns:5.1f}")
print()
for k, v in by.items():
    print(f"{k[0]:8s} t{k[1]} n={len(v)} median {statistics.median(v):6.1f} min {min(v):6.1f} max {max(v):6.1f} mean {statistics.mean(v):6.1f}")
