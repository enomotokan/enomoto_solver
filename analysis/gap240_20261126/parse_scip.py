"""SCIP の統計 (logs/scip_<問題>_<設定>.stat) から要の数を抜き出す。

使い方: python parse_scip.py 問題 [設定=def]
出す物: 前処理の後の大きさ、根の最初の LP 値と最後の双対の下界、走った回数 (再スタート)、ノード、前処理の手ごとの固定・集約・境界の変更、
分離の手ごとの採ったカット (Applied)、制約の種類ごとのカット・固定、伝播の手ごとの固定、解を出したヒューリスティクス。0 の行は出さない。"""
import re, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

def section(lines, head):
    out, on = [], False
    for l in lines:
        if l.startswith(head):
            on = True; out.append(l); continue
        if on:
            if l and not l.startswith(" "): break
            out.append(l)
    return out

def table(sec):
    hdr = sec[0].split(":", 1)[1].split()
    rows = []
    for l in sec[1:]:
        if ":" not in l: continue
        name, rest = l.split(":", 1)
        rows.append((name.strip(), rest.split()))
    return hdr, rows

def main():
    prob = sys.argv[1]; s = sys.argv[2] if len(sys.argv) > 2 else "def"
    tag = s.replace(":", "_").replace("/", "-").replace(";", "+").replace("=", "")
    L = (HERE / "logs" / f"scip_{prob}_{tag}.stat").read_text().splitlines()
    pp = section(L, "Presolved Problem")
    print(f"## {prob} [{s}]")
    for l in pp[2:7]: print(" ", l.strip())
    for l in section(L, "Root Node")[1:]: print(" ", l.strip())
    for l in section(L, "B&B Tree")[1:3]: print(" ", l.strip())
    for l in section(L, "Total Time")[:1]: print(" ", l.strip())
    hdr, rows = table(section(L, "Presolvers"))
    idx = [hdr.index(k) for k in ("FixedVars", "AggrVars", "ChgBounds", "DelCons", "AddCons", "ChgCoefs")]
    print("  前処理 (Fixed/Aggr/ChgBnd/DelCons/AddCons/ChgCoefs):",
          "; ".join(f"{n} {'/'.join(r[i] for i in idx)}" for n, r in rows if any(r[i] not in ("0", "-") for i in idx)))
    hdr, rows = table(section(L, "Separators"))
    ia = hdr.index("Applied")
    print("  分離 Applied:", ", ".join(f"{n} {r[ia]}" for n, r in rows if len(r) > ia and r[ia] not in ("0", "-")))
    hdr, rows = table(section(L, "Constraints  "))
    ic, iap, idr, ico = hdr.index("Cuts"), hdr.index("Applied"), hdr.index("DomReds"), hdr.index("Cutoffs")
    print("  制約 (Cutoffs/DomReds/Cuts/Applied):", "; ".join(f"{n} {r[ico]}/{r[idr]}/{r[ic]}/{r[iap]}" for n, r in rows
                                                       if any(r[i] not in ("0", "-") for i in (ic, iap, idr, ico))))
    hdr, rows = table(section(L, "Propagators  "))
    idr, ico = hdr.index("DomReds"), hdr.index("Cutoffs")
    print("  伝播 (Cutoffs/DomReds):", "; ".join(f"{n} {r[ico]}/{r[idr]}" for n, r in rows if r[idr] != "0" or r[ico] != "0"))
    hdr, rows = table(section(L, "Primal Heuristics"))
    i_f, i_b = hdr.index("Found"), hdr.index("Best")
    print("  解を出したヒューリスティクス (Found/Best):", ", ".join(f"{n} {r[i_f]}/{r[i_b]}" for n, r in rows if r[i_f] not in ("0", "-")))

if __name__ == "__main__":
    main()
