"""ROOTSTOP の集計。analysis/benchlib.py の report を使う。
python summ.py OUT.jsonl 試作 [既定 (def)]  ... 対の SGM・解けた回・95% 区間と、根の情報 (木に入った時刻・再スタート・前景ラウンド・裏ラウンド) の差の大きい問題"""
import json, sys
from collections import defaultdict
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import benchlib as bl  # noqa: E402
out, B = sys.argv[1], sys.argv[2]
A = sys.argv[3] if len(sys.argv) > 3 else "def"
R = bl.table(bl.load([out]))
keys = [k for k in R if A in R[k] and B in R[k]]
try:
    noise = bl.load_noise()
except Exception:  # noqa: BLE001
    noise = None
bl.report(R, A, B, keys, noise=noise, label=f"{Path(out).stem}")
root = defaultdict(dict)
skipped = defaultdict(int)
for l in open(out):
    r = json.loads(l)
    root[(r["problem"], r["rep"])][r["cfg"]] = (r.get("root") or {}, r)
print("\n根の比べ (問題 種: 時間 / 木に入った秒 / 再スタート / 前景ラウンド / 裏ラウンド)、時間か木に入った時刻が 1 割以上違う回")
n_dt = []
for k in sorted(root):
    if A not in root[k] or B not in root[k]:
        continue
    (ra, xa), (rb, xb) = root[k][A], root[k][B]
    ta, tb = bl.t(xa["result"]), bl.t(xb["result"])
    fa, fb = ra.get("tree_abs"), rb.get("tree_abs")
    if fa is not None and fb is not None:
        n_dt.append(fb - fa)
    big = abs(tb - ta) > 0.1 * max(ta, tb, 1) or (fa is not None and fb is not None and abs(fb - fa) > 0.1 * max(fa, fb, 1))
    if big:
        f = lambda rr, x, t: f"{t:6.1f} / {rr.get('tree_abs')} / {rr.get('restarts')} / {rr.get('fg_rounds_all')} / {rr.get('bg_rounds')}"
        print(f"  {k[0]:24s} s{k[1]}  {A}: {f(ra, xa, ta)}   {B}: {f(rb, xb, tb)}  (r停止 {xb.get('rs_r', 0)}, b飛ばし {xb.get('rs_b_skip', 0)})")
if n_dt:
    import statistics
    print(f"木に入った時刻の差 ({B} − {A}) の中央値 {statistics.median(n_dt):+.2f} 秒 (対 {len(n_dt)})")
