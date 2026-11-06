"""mushroom-best の再スタートの模型 (ENOMOTO_MIP_DEBUG_DUMP_RESTART で書き出したもの) を HiGHS で根だけ解き、
kCallbackMipGetCutPool で根のカットプールを取り出して、カットの形 (列の種類ごとの非零の数・係数) を数える。
使い方: python -I hcutpool.py 模型.mps 打ち切り値 出力.json"""
import collections, json, sys
import highspy
h = highspy.Highs()
h.readModel(sys.argv[1])
h.setOptionValue("threads", 4)
h.setOptionValue("mip_max_nodes", 1)
h.setOptionValue("time_limit", 120.0)
h.setOptionValue("objective_bound", float(sys.argv[2]))
pools = []
def cb(ctype, msg, dout, din, ud):
    if ctype == highspy.cb.HighsCallbackType.kCallbackMipGetCutPool:
        n = dout.cutpool_num_cut
        st = list(dout.cutpool_start); ix = list(dout.cutpool_index); va = list(dout.cutpool_value)
        lo = list(dout.cutpool_lower); up = list(dout.cutpool_upper)
        pools.append({"ncol": dout.cutpool_num_col, "ncut": n, "dual": dout.mip_dual_bound,
                      "cuts": [(ix[st[i]:st[i + 1]], va[st[i]:st[i + 1]], lo[i], up[i]) for i in range(n)]})
h.setCallback(cb, None)
h.startCallback(highspy.cb.HighsCallbackType.kCallbackMipGetCutPool)
h.run()
info = h.getInfo()
print("BOUND", info.mip_dual_bound, "pools", [(p["ncut"], p["dual"]) for p in pools])
# MIP の求解の後の getPresolvedLp は空なので、同じ模型を別に前処理だけして列の種類を取る (列の数がカットプールと同じか確かめる)
h2 = highspy.Highs(); h2.setOptionValue("output_flag", False); h2.readModel(sys.argv[1]); h2.presolve()
plp = h2.getPresolvedLp()
names = list(plp.col_names_)
integ = list(plp.integrality_)
lo, up = list(plp.col_lower_), list(plp.col_upper_)
def kind(j):
    t = "I" if int(integ[j]) == 1 else "C"
    if t == "I" and lo[j] == 0 and up[j] == 1: t = "B"
    return t
out = {"bound": info.mip_dual_bound, "npool": len(pools), "presolved": [plp.num_row_, plp.num_col_],
       "col_kinds": collections.Counter(kind(j) for j in range(plp.num_col_)), "pools": []}
for p in pools:
    sig = collections.Counter(); nnz = []
    for ix, va, l, u in p["cuts"]:
        if p["ncol"] != plp.num_col_: break
        c = collections.Counter(kind(j) for j in ix)
        sig["".join(f"{k}{c[k]}" for k in "BIC" if c[k])] += 1
        nnz.append(len(ix))
    out["pools"].append({"ncut": p["ncut"], "ncol": p["ncol"], "dual": p["dual"], "nnz_mean": sum(nnz) / max(1, len(nnz)),
                         "top_kinds": sig.most_common(15),
                         "examples": [([(names[j] if j < len(names) else j, kind(j), v, lo[j], up[j]) for j, v in zip(ix, va)], l, u) for ix, va, l, u in p["cuts"][:6]]})
json.dump(out, open(sys.argv[3], "w"), indent=1, default=str)
