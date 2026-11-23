"""perf report の Rust v0 の記号名から、読める名前 (型::関数) を取り出す簡易な後処理 (rustfilt の代わり)。
python dem.py out/<問題>.self.txt → 「割合 名前」の行。名前は記号の中の <長さ><識別子> の並びのうち、末尾の 2 つ (型・関数) をつなぐ。"""
import re, sys


def idents(sym):
    out, i = [], 0
    while i < len(sym):
        m = re.match(r"(\d+)_?", sym[i:])
        if m and m.group(1) != "0":
            n = int(m.group(1)); j = i + m.end()
            s = sym[j:j + n]
            if len(s) == n and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", s):
                out.append(s); i = j + n; continue
        i += 1
    return out


def name(sym):
    if not sym.startswith("_R"):
        return sym
    ids = [x for x in idents(sym) if x not in ("core", "alloc", "std", "llvm") and not re.fullmatch(r"[A-Z][a-z]?\d*", x)]
    skip = {"iter", "adapters", "map", "slice", "vec", "ops", "function", "impls", "traits", "iterator", "collections", "sort", "shared", "smallsort", "mip", "simplex", "spec_from_iter_nested"}
    keep = [x for x in ids if x not in skip]
    # Solver<TwoStageLp> の関数は「Solver::関数」にする (型の引数 mip_lp::TwoStageLp が間に入るため)
    if "Solver" in ids and keep:
        # TwoStageLp の後ろ (閉包・入れ子の関数名) があれば残す (例 Solver::path_aggregation::separate)
        tail = keep[keep.index("TwoStageLp") + 1:] if "TwoStageLp" in keep else keep[-1:]
        return "Solver::" + "::".join(tail[-2:] or keep[-1:])
    return "::".join(keep[-3:]) if keep else sym[:60]


def main(path):
    for l in open(path):
        m = re.match(r"\s*([\d.]+)%\s+(?:[\d.]+%\s+)?\[.\]\s+(\S+)", l)
        if m:
            print(f"{float(m.group(1)):6.2f}% {name(m.group(2))}")


if __name__ == "__main__":
    import signal
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    main(sys.argv[1])
