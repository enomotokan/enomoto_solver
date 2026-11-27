"""今回 (combo3 の第 2 段) と前回 (combo2 の第 2 段、版 c8e468e25) の比べ: python analysis/combo3_20261126/s2/vsold.py → 標準出力。
- 今の既定 def_tN / 前回の既定 def_tN (既定がどれだけ速くなったか)、今の c3_tN / 前回の def_tN、今の c3_tN / 前回の c2c_tN。
- 計算は s2/summ.py と同じ: ずらし 10 秒、解けない回は上限 600 秒で数える、問題ごとの log の平均と 95% 区間 (1.96 × 標準誤差)。種 1 だけ。
- 両方とも記録が有効な (crash・killed・取得失敗でない) 問題だけで比べる。集合は M (Mittelmann 186 問)・240 問全部・残り 54 問。
- 注意: 前回と今回は回した機械 (クラウドの入れ物) が違う。前回の行には CPU の欄がなく、同じ型の機械かは確かめられない
  (どちらも 4 コア・メモリ 15 GB)。今回の行の `cpu` 欄の内訳を最後に出す。版と機械の両方の差が混じった比として読む。"""
import glob, json, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
OLD = D.parents[1] / "combo2_20261125" / "s2"
TL, SH = 600.0, 10.0
PR = json.loads((D / "problems.json").read_text())
SETS = {"M": set(PR["M"]), "240 問全部": set(PR["all"]), "残り": set(PR["all"]) - set(PR["M"])}


def load(pat):
    R = {}
    for g in glob.glob(str(pat)):
        for l in open(g, encoding="utf-8"):
            if l.strip():
                r = json.loads(l); R[(r["problem"], r["cfg"])] = r  # 後の行 (回し直し) を取る
    return R


NEW, OLDR = load(D / "res_s*.jsonl"), load(OLD / "res_s*.jsonl")
def st(r): return str(r["result"].get("status") or "").lower()
def valid(r): return st(r) in ("optimal", "infeasible", "time_limit", "timelimit")
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"].get("time") or TL, TL) if solved(r) else TL


def cmp(a_cfg, A, b_cfg, B, th):
    print(f"\n== {th}: 今回 {a_cfg} / 前回 {b_cfg} (1 より小さければ今回が速い)\n")
    print("| 集合 | 問題 | 比 | 95% 区間 | 解けた数 前回 → 今回 | 1.25 倍超の遅れ | 0.8 倍未満の得 |")
    print("|---|---:|---:|---|---|---:|---:|")
    for name, S in SETS.items():
        ls, sa, sb, slow, fast = [], 0, 0, 0, 0
        for p in sorted(S):
            a, b = A.get((p, f"{a_cfg}_{th}")), B.get((p, f"{b_cfg}_{th}"))
            if not a or not b or not valid(a) or not valid(b): continue
            x = math.log((t(a) + SH) / (t(b) + SH)); ls.append(x)
            sa += solved(a); sb += solved(b); slow += x > math.log(1.25); fast += x < math.log(0.8)
        if len(ls) < 2: print(f"| {name} | {len(ls)} | — | — | — | — | — |"); continue
        m = sum(ls) / len(ls); se = (sum((x - m) ** 2 for x in ls) / (len(ls) - 1) / len(ls)) ** 0.5
        print(f"| {name} | {len(ls)} | {math.exp(m):.3f} | [{math.exp(m - 1.96 * se):.3f}, {math.exp(m + 1.96 * se):.3f}] | {sb} → {sa} | {slow} | {fast} |")


for th in ("t1", "t4"):
    cmp("def", NEW, "def", OLDR, th)
    cmp("c3", NEW, "def", OLDR, th)
    cmp("c3", NEW, "c2c", OLDR, th)
print("\n== 今回の行の機械 (cpu 欄の内訳、回数)\n")
for k, v in collections.Counter(r.get("cpu", "(欄なし)") for r in NEW.values()).most_common(): print(f"- {k}: {v}")
print(f"- 前回の行: {len(OLDR)} 行、cpu 欄なし (nproc {sorted(set(r.get('nproc') for r in OLDR.values()))})")
