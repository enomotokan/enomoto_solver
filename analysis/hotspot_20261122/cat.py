"""out/<問題>.self.txt (自身の時間) を段に振り分けて割合を出す: python cat.py → cats.md。振り分けは関数名の部分一致 (上から最初に当たったもの)。"""
import re, sys
from pathlib import Path
from dem import name
D = Path(__file__).resolve().parent
CATS = [
    ("双対証明 (apply_dual_proofs・proof_absorb)", r"apply_dual_proofs|proof_absorb|proof_"),
    ("衝突 (リテラル衝突のプール・解析)", r"conflict|LitConflict|analyze"),
    ("伝播 (定義域・カットプール)", r"Domain::|domain::|propagate_cut_pool|node_prop|pool_violated|propagate"),
    ("LU (分解・置換・FTRAN/BTRAN)", r"lu::|LuFactors|FtLu|basis_kernel|ftran|btran|factor"),
    ("単体法 (価格付け・比・更新)", r"slope_intercept_dual|simplex|PriceMat|dual_"),
    ("分離 (cmir・被覆・集約・選び)", r"cuts::|separation::|cmir|cover|snf|substitute|path_aggregation|select_cuts|zerohalf|clique|finish_cut|floor"),
    ("メモリ確保・複写 (malloc・free・memmove)", r"malloc|_mi_|mi_free|mi_|_int_free|_int_malloc|memmove|memcpy|memset|realloc|cfree"),
    ("ヒューリスティクス・サブ MIP", r"heur|alns|rins|rens|dive|fj|local_mip|submip"),
]


def cats(path):
    tot = {c: 0.0 for c, _ in CATS}; tot["その他"] = 0.0
    for l in open(path):
        m = re.match(r"\s*([\d.]+)%\s+(?:[\d.]+%\s+)?\[.\]\s+(\S+)", l)
        if not m: continue
        v, nm = float(m.group(1)), name(m.group(2))
        for c, pat in CATS:
            if re.search(pat, nm):
                tot[c] += v; break
        else:
            tot["その他"] += v
    return tot


probs = sys.argv[1:] or sorted(p.name[:-len(".self.txt")] for p in (D / "out").glob("*.self.txt"))
rows = {p: cats(D / "out" / f"{p}.self.txt") for p in probs}
keys = [c for c, _ in CATS] + ["その他"]
out = ["| 段 (自身の時間の %、0.3% 以上の関数の和) | " + " | ".join(probs) + " |", "|---" * (len(probs) + 1) + "|"]
for k in keys:
    out.append(f"| {k} | " + " | ".join(f"{rows[p][k]:.1f}" for p in probs) + " |")
(D / "cats.md").write_text("\n".join(out) + "\n")
print("\n".join(out))
