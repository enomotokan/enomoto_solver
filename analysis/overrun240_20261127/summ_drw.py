"""比べ (verify_drw.py) の集計: python summ_drw.py [--a] → 標準出力 (Markdown の表)。
res_drw_s*.jsonl (--a なら res_drwa_s*.jsonl) を読み、上限・スレッドごとに、各設定の result.time (秒) と返した目的値、研磨の印を並べる。
研磨の印 (POLISH_STATS=v の行から):
- pre x.xs: 前処理つきの研磨の秒。門を通った (前処理の後に小さい) ときの行。
- warm: 門で縮まず、温めた研磨へ回した。
- race P/W x.xs: 同時の研磨 (r) で、前処理つき (P) か温めた研磨 (W) が勝った秒。
- bg: 返すときに裏の研磨の結果を使った (`final polish taken from the background polish`)。
- busy k: 働いているスレッドが研磨を取った回数 (w・a)。
越え = result.time − 上限 (秒)。"""
import glob, json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A = "--a" in sys.argv
R = {}
for f in glob.glob(str(D / ("res_drwa_s*.jsonl" if A else "res_drw_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["threads"], r["tl"])] = r
AB = {"neos-5114902-kasavu": "kasavu", "neos-5049753-cuanza": "cuanza", "snp-02-004-104": "snp"}
PROBS = ["neos-5114902-kasavu", "neos-5049753-cuanza", "neos-873061", "thor50dday", "dano3_5", "snp-02-004-104"]
CF = ["Ra", "Rw"] if A else ["A", "D", "R", "Ri", "Rw"]
def marks(r):
    m = []
    for l in r.get("polish", []):
        if "POLISH_RACE:" in l:
            w = l.split("POLISH_RACE: ")[1]
            m.append(("race P " if w.startswith("presolve") else "race W " if w.startswith("warm won") else w.split(" ")[0] + " ") + w.split(" ")[-3] if "won" in w else "race " + w.split(" ->")[0])
        elif "final polish taken from the background" in l:
            m.append("bg")
        elif "not smaller after presolve" in l:
            m.append("warm")
        elif "POLISH_PRESOLVE: Optimal" in l and "gate: smaller" in l:
            m.append("pre " + l.split("Optimal ")[1].split("s ")[0] + "s")
        elif "taken by busy threads" in l:
            k = l.split("taken by busy threads ")[1].strip()
            if k != "0":
                m.append("busy " + k)
    return " ".join(dict.fromkeys(m))
def cell(r):
    if r is None:
        return "—"
    x = r["result"]
    if x.get("status") == "killed":
        return f"止めた ({r['wall']:.0f})"
    if x.get("status") == "crash":
        return "crash"
    over = x["time"] - r["tl"]
    s = "最適 " if x["status"] == "optimal" else f"+{over:.1f} "
    return f"{s}{x['objective']:.10g}" + (f" [{marks(r)}]" if marks(r) else "")
print(f"行 {len(R)}")
for tl in (120, 600):
    ths = sorted({k[2] for k in R if k[3] == tl})
    for th in ths:
        cs = [c for c in CF if any((p, c, th, tl) in R for p in PROBS)]
        if not cs:
            continue
        print(f"\n### 上限 {tl} 秒、{th} スレッド (越えの秒と返した目的値)\n")
        print("| 問題 | " + " | ".join(cs) + " |")
        print("|---|" + "---|" * len(cs))
        for p in PROBS:
            print(f"| {AB.get(p, p)} | " + " | ".join(cell(R.get((p, c, th, tl))) for c in cs) + " |")
