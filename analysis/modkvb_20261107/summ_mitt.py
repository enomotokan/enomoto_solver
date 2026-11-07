"""Mittelmann 186 問の 60 秒 (bmitt.jsonl、def6・mk50z、種 1・2) の比べ: benchlib の report、片方だけ解けた問題、根の下界が (2 回とも) 動いた問題、
試作の候補の多い問題と根のカットの LP が反復の上限に当たったか (logs の「per-LP cap」)、zerohalf との平行の数。使い方: python summ_mitt.py"""
import gzip, json, sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl  # noqa: E402
rows = [json.loads(l) for l in open(here / "bmitt.jsonl")]
R = bl.table(bl.load([here / "bmitt.jsonl"]))
ks = [k for k in R if "def6" in R[k] and "mk50z" in R[k]]
reps = sorted({k[1] for k in ks})
for rep in reps:
    kk = [k for k in ks if k[1] == rep]
    print(f"種 {rep}: {len(kk)} 問  def6 解けた {sum(bl.ok(R[k]['def6']) for k in kk)} SGM {bl.sgm([bl.t(R[k]['def6']) for k in kk]):.2f} / "
          f"mk50z 解けた {sum(bl.ok(R[k]['mk50z']) for k in kk)} SGM {bl.sgm([bl.t(R[k]['mk50z']) for k in kk]):.2f}")
bl.report(R, "def6", "mk50z", ks, label="Mittelmann def6→mk50z", show_root=True)
rec = {(r["problem"], r["rep"], r["cfg"]): r for r in rows}
def caphit(cfg, p, rep):
    f = here / "logs" / f"{cfg}_{p}_60_r{rep}.err.gz"
    if not f.exists():
        return None
    return sum(1 for l in gzip.open(f, "rt", errors="replace") if "per-LP cap" in l)
print("\n片方だけ解けた回 (問題 種 def6 の時間 mk50z の時間):")
for k in sorted(ks):
    a, b = R[k]["def6"], R[k]["mk50z"]
    if bl.ok(a) != bl.ok(b):
        print(f"  {k[0]:28s} {k[1]} {bl.t(a):6.1f} {bl.t(b):6.1f}")
print("\n根の最初のループの下界が動いた問題 (種ごと、相対 1e-6 超え; 2 回とも同じ向きなら *):")
mv = {}
for k in ks:
    a = rec.get((k[0], k[1], "def6"), {}).get("root_loops") or []
    b = rec.get((k[0], k[1], "mk50z"), {}).get("root_loops") or []
    if a and b:
        x, y = a[0]["obj1"], b[0]["obj1"]
        if abs(x - y) > 1e-6 * max(1, abs(x)):
            mv.setdefault(k[0], []).append((k[1], x, y))
for p, L in sorted(mv.items()):
    same = len(L) == len(reps) > 1 and len({(y > x) for _, x, y in L}) == 1
    print(f"  {p:28s}{'*' if same else ' '} " + "  ".join(f"種{r}: {x:.6g} → {y:.6g}" for r, x, y in L))
print("\n試作の候補の多い問題 (種 1、上位 15): 問題 候補 分離の秒 zerohalf と平行 / 残した 根のカットの LP の上限 (def6 / mk50z)")
top = sorted([r for r in rows if r["cfg"] == "mk50z" and r["rep"] == reps[0]], key=lambda r: -r.get("sep", {}).get("cands", 0))[:15]
for r in top:
    z = r.get("zhcmp", [0] * 9)
    print(f"  {r['problem']:28s} {r['sep'].get('cands')} {r['sep'].get('time', 0):.2f} {z[2] + z[6]}/{z[1] + z[5]}  {caphit('def6', r['problem'], r['rep'])} / {caphit('mk50z', r['problem'], r['rep'])}")
