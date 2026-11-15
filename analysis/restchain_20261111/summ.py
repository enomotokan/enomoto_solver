"""60 秒の比べの集計 (benchlib.report): python summ.py res58.jsonl 設定[,設定] [問題の一覧ファイル]
加えて、最初の暫定解の時刻 (前処理の始めから、秒) と前処理の時間の比べを出す。"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import benchlib as bl  # noqa: E402
res = HERE / sys.argv[1]
rows = bl.load([res])
R = bl.table(rows)
noise = bl.load_noise()
only = set(open(sys.argv[3]).read().split()) if len(sys.argv) > 3 else None
raw = {}
for l in open(res):
    r = json.loads(l)
    raw[(r["problem"], r["rep"], r["cfg"])] = r
def first_inc(r):
    if r.get("inc") is None or r.get("pre") is None:
        return None
    return r["pre"] + r["inc"]
for B in sys.argv[2].split(","):
    keys = sorted(k for k in R if "def" in R[k] and B in R[k] and (only is None or k[0] in only))
    bl.report(R, "def", B, keys, noise=noise, label=f"def vs {B} ({len(keys)} 回)")
    # 最初の暫定解の時刻 (なしは 60 秒) と前処理の時間 (シフト 1 秒の幾何平均)
    a_inc, b_inc, a_pre, b_pre, faster, slower = [], [], [], [], [], []
    for (p, rp) in keys:
        ra, rb = raw.get((p, rp, "def")), raw.get((p, rp, B))
        if not ra or not rb:
            continue
        ia, ib = first_inc(ra), first_inc(rb)
        a_inc.append(ia if ia is not None else 60.0)
        b_inc.append(ib if ib is not None else 60.0)
        if ra.get("pre") is not None and rb.get("pre") is not None:
            a_pre.append(ra["pre"]); b_pre.append(rb["pre"])
        if ia is not None and ib is not None:
            if ib < ia - 0.5:
                faster.append(f"{p} {ia:.1f}->{ib:.1f}")
            elif ib > ia + 0.5:
                slower.append(f"{p} {ia:.1f}->{ib:.1f}")
        elif (ia is None) != (ib is None):
            (faster if ia is None else slower).append(f"{p} {ia}->{ib}")
    g = lambda xs: math.exp(sum(math.log(x + 1) for x in xs) / len(xs)) - 1 if xs else float("nan")
    print(f"最初の暫定解の時刻 (前処理の始めから、シフト 1 の幾何平均): def {g(a_inc):.2f}s / {B} {g(b_inc):.2f}s ({len(a_inc)} 回)")
    print(f"前処理の時間 (PRETIME total、シフト 1 の幾何平均): def {g(a_pre):.2f}s / {B} {g(b_pre):.2f}s ({len(a_pre)} 回)")
    print(f"  0.5 秒以上早い: {len(faster)}: " + ", ".join(faster))
    print(f"  0.5 秒以上遅い: {len(slower)}: " + ", ".join(slower))
    print()
