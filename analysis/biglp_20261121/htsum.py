"""heaptrack_print の PEAK MEMORY CONSUMERS の節を、確保の場所ごとに縮めて上位を出す (リリースのビルド、行番号なし)。
Rust v0 の名前 (_RNv...) は「長さ+識別子」の並びから識別子を拾い、各枠の最後の 1〜2 個 (型・関数) で表す。alloc の枠は飛ばす。
使い方: heaptrack_print -f logs/rail01_ht.gz > ht.txt; python htsum.py ht.txt [上位の数]"""
import re, sys
L = open(sys.argv[1], errors="replace").read().splitlines()
top = int(sys.argv[2]) if len(sys.argv) > 2 else 15
s = next(i for i, l in enumerate(L) if l.startswith("PEAK MEMORY CONSUMERS"))
e = next(i for i, l in enumerate(L) if l.startswith("MOST TEMPORARY ALLOCATIONS"))
SKIP = {"alloc", "raw_vec", "RawVecInner", "RawVec", "finish_grow", "grow_one", "grow_amortized", "core", "std", "vec", "Vec", "boxed", "clone",
        "spec_from_elem", "from_elem", "extend_with", "to_vec", "cancel", "with_halt", "sys", "backtrace", "thread", "lifecycle", "start_thread"}
def idents(sym):
    out = []; i = 0
    while i < len(sym):
        m = re.match(r"(\d+)", sym[i:])
        if m:
            n = int(m.group(1)); j = i + len(m.group(1))
            if j < len(sym) and sym[j] == "_": j += 1
            w = sym[j:j + n]
            if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", w or "-") and not re.search(r"\d{3,}|^Cs|^_[A-Z]", w): out.append(w)
            i = j + max(n, 1)
        else: i += 1
    return out
def mb(x):
    m = re.match(r"([0-9.]+)([KMG]?)", x); return float(m.group(1)) * {"": 1e-6, "K": 1e-3, "M": 1, "G": 1e3}[m.group(2)]
blocks = []
for l in L[s:e]:
    m = re.match(r"^\s*([0-9.]+[KMG]?)B? (?:peak memory )?consumed over (\d+) calls from", l)
    if m: blocks.append([mb(m.group(1)), int(m.group(2)), [], "peak memory" not in l]); continue
    if blocks and re.match(r"^\s+_R", l):
        ids = [w for w in idents(l.strip()) if w not in SKIP and w not in ("_core", "mip", "simplex", "mip_lp", "TwoStageLp", "solver", "Solver")]
        if ids:
            n = ".".join(ids[-2:])
            if n not in blocks[-1][2] and len(blocks[-1][2]) < 5: blocks[-1][2].append(n)
sub = [b for b in blocks if b[3]]
sub.sort(key=lambda b: -b[0])
print(f"呼び出し元ごと (字下げの行) の合計 {sum(b[0] for b in sub):.0f} MB ({len(sub)} 箇所)")
for b in sub[:top]:
    print(f"{b[0]:8.1f} MB  {b[1]:7d} 回  " + " ← ".join(b[2]))
