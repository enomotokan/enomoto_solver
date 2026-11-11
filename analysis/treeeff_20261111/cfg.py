"""TREEEFF (2026-11-11) の設定。def = 既定 (環境変数なし)。診断 (D) は既存の試作の組み合わせで、木のノード数の差の元を切り分ける。
highs = HiGHS 1.15.1 (種は random_seed)。"""
TRF = {"ENOMOTO_MIP_TREE_RESTART": "force", "ENOMOTO_MIP_TREE_RESTART_GUARD": "KR"}  # (1) 木の中の再スタート (HiGHS 式の判定、固定 0 本でも。損の型を KR で見送る。analysis/tree_restart_guard_20261106.md §12 の最良の形)
NC = {"ENOMOTO_MIP_NODECNT": "hr"}                       # (2) ノードでも経路集約 (根で採った本数で絞る形、analysis/nodecnt_20261107.md 9 節)
SD = {"ENOMOTO_MIP_SB_NO_DEGEN_SKIP": "1"}               # (3) 双対退化でも強分岐を飛ばさない
PL = {"ENOMOTO_MIP_TREE1_POOL": "1"}                     # (5) 4 スレッドの木: 下界最小を取る番に共有の待ち行列も見る (analysis/tree_close1_20261104.md)
CA = {"ENOMOTO_MIP_CUT_AGE_HIGHS": "1", "ENOMOTO_T_MIP_CUT_ROWS_CAP": "1"}  # (4) HiGHS 式の年齢 + 木の LP のカットの行を max(問題の行, 200) まで
CFGS = {
    "def": {},
    "trf": TRF,
    "nc": NC,
    "sd": SD,
    "ca": CA,
    "pl": PL,
    "all": {**TRF, **NC, **SD, **CA, **PL},
}
# 診断の回でだけ足すログ (解き方は変えない)
LOGENV = {"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_TREE_RESTART_LOG": "1", "ENOMOTO_MIP_TREE4_TRACE": "10"}
