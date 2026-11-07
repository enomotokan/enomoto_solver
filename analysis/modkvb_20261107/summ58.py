"""58 問の 60 秒 (b58.jsonl、def・mk、種 1・2) の比べ (benchlib の report) と、試作の分離の候補・時間。使い方: python summ58.py"""
import json, sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "b58.jsonl"]))
for rep in (1, 2):
    ks = [k for k in R if k[1] == rep and 'def' in R[k] and 'mk' in R[k]]
    if not ks: continue
    print(f"回 {rep}: def SGM {bl.sgm([bl.t(R[k]['def']) for k in ks if 'def' in R[k] and 'mk' in R[k]]):.2f} 解けた {sum(bl.ok(R[k]['def']) for k in ks if 'mk' in R[k])}"
          f" / mk SGM {bl.sgm([bl.t(R[k]['mk']) for k in ks if 'def' in R[k] and 'mk' in R[k]]):.2f} 解けた {sum(bl.ok(R[k]['mk']) for k in ks if 'def' in R[k])} ({len([k for k in ks if 'def' in R[k] and 'mk' in R[k]])} 問)")
bl.report(R, "def", "mk", list(R), noise=bl.load_noise(), label="58 問 def→mk")
if any("mk50" in d for d in R.values()):
    bl.report(R, "def", "mk50", list(R), noise=bl.load_noise(), label="58 問 def→mk50 (候補 50 本まで)")
    bl.report(R, "mk", "mk50", list(R), noise=bl.load_noise(), label="58 問 mk→mk50")
# 時間が 1.5 倍以上違う・解けた/解けないが変わった回
print("\n1.5 倍以上・解けたかが変わった回 (問題 回 def mk、mk の分離の候補・時間):")
sep = {}
for l in open(here / "b58.jsonl"):
    r = json.loads(l)
    if r["cfg"] == "mk": sep[(r["problem"], r["rep"])] = r.get("sep", {})
    if r["cfg"] == "mk50": sep50 = sep.setdefault(("mk50",), {}); sep50[(r["problem"], r["rep"])] = r
for k in sorted(R):
    d = R[k]
    if "def" not in d or "mk" not in d: continue
    a, b = bl.t(d["def"]), bl.t(d["mk"])
    if bl.ok(d["def"]) != bl.ok(d["mk"]) or max(a + 10, b + 10) / min(a + 10, b + 10) > 1.5:
        s = sep.get(k, {})
        print(f"  {k[0]:22s} {k[1]} {a:6.1f} {b:6.1f}  cands {s.get('cands')} t {s.get('time', 0):.2f}")
