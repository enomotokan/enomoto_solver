"""REVISIT (analysis/revisit_20261112.md) の設定。以前に既定化を見送った試作を、今の既定 (環境変数なし) と対にして測り直す。
どれも既定では使わない試作で、今のソースで環境変数が読まれていることを確かめた (2026-11-12)。"""
CFGS = {
    "def": {},
    # GAP4 の q (analysis/gap4_rootstuck_mem_20261109.md: 58 問 −2.0% [−6.2%, +1.6%]、解けた 83→87)
    "g4q": {"ENOMOTO_MIP_GAP4": "q"},
    # HiGHS の規則の表の分離 (同じ記録: 58 問 −4.2% [−10.3%, +2.3%]、87→87、メモリ −12%)
    "tab": {"ENOMOTO_MIP_TAB_HIGHS_RULES": "1"},
    # Local-MIP の l (analysis/lmip2_bt_20261105.md: 58 問 4 回とも −1〜−5%、116→115)
    "lm2": {"ENOMOTO_MIP_LMIP2": "l"},
    # サブ MIP の上限 JT (analysis/submip_cap_20261106.md: 58 問 −1.4% [−3.8%, +1.2%]、84→83、600 秒の trento1・thor50dday)
    "subcap": {"ENOMOTO_MIP_SUBCAP": "JT"},
    # HNF の合同式のカット bc (analysis/hnfcut_20261107.md: 58 問 +1.7% [−1.5%, +5.5%]、avoca・rococoB10)
    "hnfbc": {"ENOMOTO_MIP_HNFCUT": "bc"},
    # 縮約の分割カット b (analysis/redsplit_20261107.md: 58 問 −0.8% [−6.4%, +4.9%]、86→87、根の下界が良い)
    "rs": {"ENOMOTO_MIP_REDSPLIT": "b", "ENOMOTO_T_MIP_REDSPLIT_TFRAC": "0.05"},
    # 並列のプロービング (analysis/prepar_20261107.md: 前処理の結果は同じ、58 問 +1.1% [−1.4%, +4.2%]、重い問題で前処理 −10%)
    "prepar": {"ENOMOTO_MIP_PREPAR": "ple"},
    # 共有の待ち行列の取り出し (analysis/pool_bound2_20261104.md: 58 問 1 回 −2.6%、44→44、最大 RSS 11.6→6.9 GB)
    "pool2": {"ENOMOTO_MIP_POOL2_TAKE": "g,i,k4"},
    # 最適面の並列のカット 3 (analysis/optface_cuts_20261105.md: 解けた 39→42・39→40)
    "opf3": {"ENOMOTO_MIP_OPTFACE_PARCUTS": "3"},
    # 強分岐の締め (analysis/cbcimpl_20261029.md の sbt_abpr: 58 問 ×3 −1.3%、一般整数 16 問 −4.9%、Mittelmann −1.2%)
    "sbt": {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1",
            "ENOMOTO_MIP_SB_POLY_RC": "1"},
    # ノードのカットの長さの上限 (analysis/cbcimpl_20261029.md: 58 問 ×2 −1.2%、81→85)
    "ncm": {"ENOMOTO_MIP_NODE_CUT_MAXLEN": "1"},
    # 解のないサブ MIP の上限を直列のときだけ (analysis/submip_nodes_20261101.md の ns250s。当時は h80x6320d の前処理の誤りの道を開いたので見送り、誤りは後で直った)
    "ns250s": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250", "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL": "1"},
    # 面の LP の適応 (analysis/sepspeed_20261023.md: 10teams 27→5 秒、単独では未測定)
    "face": {"ENOMOTO_MIP_FACE_ADAPT": "1"},
}
