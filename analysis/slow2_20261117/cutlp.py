"""カットの集合の強さの比べ: HiGHS の前処理後の問題 (MPS) の LP 緩和に、カットの集合を足した LP の値を出す。
python cutlp.py <mps> <HiGHS のカット (hcutsrc の書き出し)> <当方の候補のディレクトリ (DUMP_CANDS)>
集合: HiGHS の全部・出どころごと・当方の候補の全部 (cands_r*)・当方の採ったもの (chosen_r*)・当方の候補の出どころごと。"""
import sys, glob, collections, highspy, numpy as np
mps, hc, ed = sys.argv[1:4]
def load_h(src=None):
    out = []
    for l in open(hc):
        t = l.split()
        if src is not None and int(t[1]) not in src: continue
        out.append((float(t[2]), [(int(a.split(":")[0]), float(a.split(":")[1])) for a in t[3:]]))
    return out
def load_e(pat, src=None):
    out = []
    for f in glob.glob(f"{ed}/{pat}"):
        for l in open(f):
            t = l.split()
            if src is not None and int(t[0]) not in src: continue
            out.append((float(t[2]), [(int(a.split(":")[0]), float(a.split(":")[1])) for a in t[3:]]))
    return out
def lpval(cuts):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps)
    lp = h.getLp(); n = lp.num_col_
    h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
    if cuts:
        lo = np.full(len(cuts), -highspy.kHighsInf); up = np.array([c[0] for c in cuts])
        st = [0]; idx = []; val = []
        for _, co in cuts:
            idx += [j for j, _ in co]; val += [a for _, a in co]; st.append(len(idx))
        h.addRows(len(cuts), lo, up, len(idx), np.array(st[:-1], dtype=np.int32), np.array(idx, dtype=np.int32), np.array(val))
    h.run(); return h.getInfo().objective_function_value, h.modelStatusToString(h.getModelStatus())
for name, cuts in [("なし", []), ("HiGHS 全部", load_h()), ("HiGHS implbd", load_h({0})), ("HiGHS path", load_h({3})), ("HiGHS tableau", load_h({2})),
                   ("HiGHS implbd+path", load_h({0, 3})), ("当方 候補 全部", load_e("cands_r*.txt")), ("当方 採用 全部", load_e("chosen_r*.txt")),
                   ("当方 候補 implbd(4)", load_e("cands_r*.txt", {4})), ("当方 候補 pathH(5)", load_e("cands_r*.txt", {5})), ("当方 候補 path(6)", load_e("cands_r*.txt", {6})),
                   ("当方 候補 tab(7)", load_e("cands_r*.txt", {7})), ("当方 候補 4+5+6", load_e("cands_r*.txt", {4, 5, 6}))]:
    v, s = lpval(cuts)
    print(f"{name}: {len(cuts)} 本 → LP {v:.6e} ({s})", flush=True)
