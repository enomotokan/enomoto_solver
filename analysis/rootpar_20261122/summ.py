"""rootpar の集計: python summ.py > summ.txt (res.jsonl)。
段 (pre・lp・cut・heur・post、再スタートごとの分は足す) ごとに、1 スレッドと 4 スレッドの秒、4 スレッドの段の CPU / 秒 (平均で何本動いたか)、
4/1 の秒の比。全体では段の秒の合計 (問題を足した) と割合、根 (pre〜post) の合計に占める割合。
pre は起動からの壁の秒で測るので、起動〜求解の始め (壁の秒 - 結果の time) を引く (その間の CPU はそのまま数える)。\nアムダールの見積り: 4 スレッドの根の秒のうち段 X が CPU / 秒 ≈ c で動いているとき、X を k 本 (k=4) で完全に並列にできれば X の秒は c/k 倍になる
(今の CPU の総量が同じと仮定)。段ごとに「その段だけ 4 本にしたときの根の秒の減り」を出す。"""
import json, collections
R = {}
for l in open("res.jsonl"):
    if l.strip():
        r = json.loads(l); R[(r["problem"], r["threads"])] = r
PH = ["pre", "lp", "cut", "heur", "post"]
probs = sorted({p for p, t in R if (p, 1) in R and (p, 4) in R})
def load(r): return max(0.0, r["wall"] - (r["result"].get("time") or r["wall"]))
def sec(r, k):
    """pre からは起動〜求解の始め (Python の読み込み・モデルの組み立て = 壁の秒 - 求解の秒) を引く"""
    v = r["phases"].get(k, {}).get("secs", 0.0)
    return max(0.0, v - load(r)) if k == "pre" else v
def cpu(r, k): return r["phases"].get(k, {}).get("cpu", 0.0)
def root(r): return sum(sec(r, k) for k in PH) + (sec(r, "rest") if "tree" not in r["phases"] else 0.0)
print(f"問題 {len(probs)} (種 1、120 秒、ENOMOTO_MIP_LOG=1)")
print("\n## 問題ごと (秒: 1 スレッド / 4 スレッド [4 スレッドの CPU/秒])")
print("| 問題 | 状態 1/4 | 全体 1/4 | 根 1/4 | 求解の数 1/4 | pre | lp | cut | heur | post | 木 |")
print("|" + "---|" * 11)
T = collections.defaultdict(lambda: [0.0, 0.0, 0.0])  # 段 -> [1T 秒, 4T 秒, 4T CPU]
for p in probs:
    a, b = R[(p, 1)], R[(p, 4)]
    cells = []
    for k in PH + ["tree"]:
        s1, s4, c4 = sec(a, k), sec(b, k), cpu(b, k)
        T[k][0] += s1; T[k][1] += s4; T[k][2] += c4
        cells.append(f"{s1:.1f}/{s4:.1f} [{c4 / s4:.1f}]" if s4 > 0.05 else f"{s1:.1f}/{s4:.1f}")
    st = f"{a['result'].get('status', '?')[:4]}/{b['result'].get('status', '?')[:4]}"
    print(f"| {p} | {st} | {a['wall']:.1f}/{b['wall']:.1f} | {root(a):.1f}/{root(b):.1f} | {a['segments']}/{b['segments']} | " + " | ".join(cells) + " |")
r1 = sum(T[k][0] for k in PH); r4 = sum(T[k][1] for k in PH)
print(f"\n## 合計 ({len(probs)} 問を足した秒)")
print(f"根: 1 スレッド {r1:.0f} 秒、4 スレッド {r4:.0f} 秒 (4/1 = {r4 / r1:.2f})。木: {T['tree'][0]:.0f} → {T['tree'][1]:.0f} 秒")
print("| 段 | 1T 秒 | 4T 秒 | 4T/1T | 4T の根に占める割合 | 4T の CPU/秒 | その段だけ 4 本で完全に並列にしたときの 4T の根の秒の減り |")
print("|---|---|---|---|---|---|---|")
for k in PH:
    s1, s4, c4 = T[k]
    par = c4 / s4 if s4 > 0 else 0.0
    gain = s4 - s4 * min(1.0, max(par, 1.0) / 4) if s4 > 0 else 0.0
    print(f"| {k} | {s1:.0f} | {s4:.0f} | {s4 / s1 if s1 else 0:.2f} | {s4 / r4:.2f} | {par:.2f} | {gain:.0f} 秒 ({gain / r4:.0%}) |")
# 根のヒューリスティクスの名前ごと (4T の秒の合計)
H1 = collections.Counter(); H4 = collections.Counter()
for p in probs:
    for k, v in R[(p, 1)]["root_heur"].items(): H1[k] += v
    for k, v in R[(p, 4)]["root_heur"].items(): H4[k] += v
print("\n## 根のヒューリスティクス (名前ごとの秒の合計、1T / 4T、多い順 12)")
for k, v in H4.most_common(12):
    print(f"- {k}: {H1[k]:.1f} / {v:.1f}")
