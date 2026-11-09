"""カットの LP の本数を半分にする試作 (`ENOMOTO_MIP_CUT_LP_HALVE`) と HiGHS 風のサブ MIP の形を、4 問で試す (2026-10-30、4 スレッド・60 秒)。
使い方: python run.py <設定名,...> <問題,...> <回数>。ログは <問題>_<設定>_<回>.log。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {
    # 2026-10-31 (`analysis/cand_default_20261031.md`) から、"def" (環境変数なし) は下の "cand" と同じ形 (cand の 7 つの設定が既定)。
    # それより前の計測の "def" は旧既定で、新しいビルドで旧既定を回すには "olddef" を使う
    "def": {},
    "olddef": {"ENOMOTO_MIP_SUBMIP_HIGHS": "0", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "0", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "-1", "ENOMOTO_MIP_BG_CUTS_WAIT": "0", "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "0", "ENOMOTO_T_MIP_RESTART_FIRST_FAC": "0", "ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "0"},
    "halve": {"ENOMOTO_MIP_CUT_LP_HALVE": "1"},
    "hnt": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "h1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "rhl": {"ENOMOTO_MIP_RENS_HIGHS": "1", "ENOMOTO_MIP_DIVE_LOCK_ORDER": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "halve_hnt": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "halve_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "frac04": {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "halve_frac04": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "hm3": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3"},
    "hm3_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "hm3_frac04": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "hm3_cfl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    "hm3_cfl_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "hm3_frac04_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "dbl": {"ENOMOTO_MIP_CUT_WIDEN_DOUBLE": "1"},
    "hm3_cfl_dbl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_CUT_WIDEN_DOUBLE": "1"},
    "hs": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"},
    "hm3_cfl_hs": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-s"},
    "bgw": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "bgw05": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1", "ENOMOTO_T_MIP_BG_CUTS_WAIT_FRAC": "0.05"},
    "hm3_cfl_hs_bgw": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "hs_r1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "1"},
    "hs_pi0": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"},
    "hs_pi1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "1"},
    "hs_pi0_bgw": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "hs_pi0_bgw_r": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_BG_CUTS_WAIT": "1", "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r"},
    # 既定にする候補 (2026-10-31): hs_pi0_bgw_r + 初回の再スタートの閾値 1% + 裏のカットの証明で根のヒューリスティクスを止める
    "cand": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_BG_CUTS_WAIT": "1", "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r", "ENOMOTO_T_MIP_RESTART_FIRST_FAC": "0.01", "ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "1"},
    # サブ MIP のノード上限 (多くは 500) を下げる (2026-11-01、新しい既定の上で)
    "nm02": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT": "0.2"},
    "nm01": {"ENOMOTO_T_MIP_SUBMIP_NODES_MULT": "0.1"},
    # サブ MIP の無駄を削る試作 (2026-11-01、新しい既定の上で。analysis/rens_more_real・rens_depth3・rens_resume_20261101.md)
    "real": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    "d2lp1": {"ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
    "trim3": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"},
    # trim3 + 解なしのサブ MIP を止める (t20、analysis/submip_nosol_stop_20261101.md)
    "trim3t20": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1", "ENOMOTO_MIP_SUBMIP_NOSOL_STOP": "1", "ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES": "100", "ENOMOTO_T_MIP_SUBMIP_NOSOL_TIME_FRAC": "0.2"},
    # trim3 の既定化 (2026-11-01、analysis/trim3_default_20261101.md) の後は "def" が trim3 を指す。それより前の既定 (cand) は "pretrim3"
    "pretrim3": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "0", "ENOMOTO_MIP_RENS_RESUME": "0", "ENOMOTO_MIP_RENS_MAX_DEPTH": "1000", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1000"},
    # RENS の固定の順を HiGHS と同じ lock の順にする (2026-11-01、trim3 の既定化の後。analysis/neos960392_20261101.md の 2 節)
    "lock": {"ENOMOTO_MIP_DIVE_LOCK_ORDER": "1"},
    "lock06": {"ENOMOTO_MIP_DIVE_LOCK_ORDER": "1", "ENOMOTO_T_MIP_NOINC_RENS_RATE": "0.6"},
    "rhlock": {"ENOMOTO_MIP_DIVE_LOCK_ORDER": "1", "ENOMOTO_MIP_RENS_HIGHS": "1"},
    # neos-960392 の対策 (2026-11-01、analysis/neos960392_cutstop_20261101.md・neos960392_nested_20261101.md)
    "cs1": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat"},
    "cs2": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    "nesti": {"ENOMOTO_MIP_SUBMIP_NESTED": "i", "ENOMOTO_T_MIP_SUBMIP_NESTED_TIME_MAX": "0.5", "ENOMOTO_T_MIP_SUBMIP_NESTED_NODES": "50"},
    # cs2 の既定化 (2026-11-01、analysis/cs2_default_20261101.md) の後は "def" が cs2 を含む。それより前の既定は "precs2"
    "precs2": {"ENOMOTO_T_MIP_CUT_LP_WORK": "0", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "0", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "0", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "0"},
    # ns1116954・physiciansched6-2 の手当て (2026-11-02、analysis/ns1116954_20261101.md・phys6_20261101.md)
    "fjearly": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "1"},
    "bgx": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "lpw"},
    "fjbgx": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "1", "ENOMOTO_MIP_BG_CUTS_BASIS_X": "lpw"},
    # 目的関数 0 の早めの探索を FJ 3 本 + Local-MIP 1 本にし、裏のカットの写しに列の値を渡す (2026-11-02、analysis/feas_early_lmip_20261102.md・phys6_20261101.md)
    # flbgx の既定化 (2026-11-02、analysis/flbgx_default_20261102.md) の後は "def" が flbgx を含む。それより前の既定は "preflbgx"
    "preflbgx": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "0", "ENOMOTO_MIP_BG_CUTS_BASIS_X": "0"},
    "flbgx": {"ENOMOTO_MIP_FEAS_FJ_EARLY": "fl", "ENOMOTO_T_MIP_FEAS_EARLY_LMIP_THREADS": "1", "ENOMOTO_MIP_BG_CUTS_BASIS_X": "lpw"},
    # 特異基底の修正 C (2026-11-02、analysis/singular_fix_c_20261102.md): 根・再スタートの後の根に特異でない基底を渡す。rbfs は再スタートの後の根だけ
    "rbf": {"ENOMOTO_MIP_ROOT_BASIS_FIX": "1"},
    "rbfs": {"ENOMOTO_MIP_ROOT_BASIS_FIX": "s"},
    # 裏のカットのスレッドの LP の上限と DSE 重み・共有の待ち行列のノードの基底 (2026-11-02、analysis/bgx_check1_20261102.md・bgdse_20261102.md・singular_fix_b_20261102.md)
    "bgcap": {"ENOMOTO_MIP_BG_CUTS_LP_CAP": "1"},
    "bgdsecap": {"ENOMOTO_MIP_BG_CUTS_DSE": "r", "ENOMOTO_MIP_BG_CUTS_LP_CAP": "1"},
    "poolfix": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1"},
    # DSE 重みの受け渡し (2026-11-02、analysis/pool_dse_20261102.md・bg_return_dse_20261102.md)
    "pooldse": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "1"},
    "bgret": {"ENOMOTO_MIP_BG_RETURN_DSE": "1"},
    # 既定の候補と裏のカットの運を除く試作・LP による固定の順番 (2026-11-02、bench18。analysis/bg_determ_20261102.md・bg_lp_work_20261102.md・lp_probe_after_ext_20261102.md・small_overhead2_20261102.md)
    # canddet の既定化 (2026-11-02、analysis/canddet_default_20261102.md) の後は "def" が canddet を含む。それより前の既定は "precanddet"。
    # 注: 下の "cand" は 40 行目の 2026-10-31 の "cand" と同じ名前で、辞書では後ろのこちらが使われる (bench18 はこちらで測った)
    "precanddet": {"ENOMOTO_MIP_POOL_BASIS_FIX": "0", "ENOMOTO_MIP_POOL_DSE": "0", "ENOMOTO_MIP_SMALL2_FJSTOP": "0", "ENOMOTO_MIP_BG_CUTS_LP_WORK": "0", "ENOMOTO_MIP_BG_CUTS_DETERM": "0"},
    "cand": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "1", "ENOMOTO_MIP_SMALL2_FJSTOP": "1"},
    "canddet": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "1", "ENOMOTO_MIP_SMALL2_FJSTOP": "1", "ENOMOTO_MIP_BG_CUTS_LP_WORK": "1", "ENOMOTO_MIP_BG_CUTS_DETERM": "c"},
    "canddetext": {"ENOMOTO_MIP_POOL_BASIS_FIX": "1", "ENOMOTO_MIP_POOL_DSE": "1", "ENOMOTO_MIP_SMALL2_FJSTOP": "1", "ENOMOTO_MIP_BG_CUTS_LP_WORK": "1", "ENOMOTO_MIP_BG_CUTS_DETERM": "c", "ENOMOTO_MIP_LP_PROBE_AFTER_EXT": "1"},
    # canddet の既定化の後に厳密な DSE 重みを 58 問・Mittelmann で測り直す (2026-11-02、bench19。analysis/bgdse_grow_20261102.md・bg_lp_work_20261102.md)
    "dserw": {"ENOMOTO_MIP_BG_CUTS_DSE": "rw"},
    "dseg": {"ENOMOTO_MIP_BG_CUTS_DSE": "g"},
    # 道の変化に強くする試作 (2026-11-03、bench20。analysis/robust_cap_20261102.md・robust_stall_20261102.md)
    "p": {"ENOMOTO_MIP_CUT_LP_CAP_ROBUST": "p"},
    # p の既定化 (2026-11-03、analysis/robust_cap_default_20261103.md) の後は "def" が p を含む。それより前の既定は "prep"
    "prep": {"ENOMOTO_MIP_CUT_LP_CAP_ROBUST": "0"},
    "po": {"ENOMOTO_MIP_CUT_LP_CAP_ROBUST": "p", "ENOMOTO_MIP_BG_DETERM_ROBUST": "o"},
    "pog": {"ENOMOTO_MIP_CUT_LP_CAP_ROBUST": "p", "ENOMOTO_MIP_BG_DETERM_ROBUST": "o", "ENOMOTO_MIP_BG_CUTS_DSE": "g"},
    # HiGHS に負けた 6 問の試作 (2026-11-03、bench21。analysis/lose6_1_20261103.md・lose6_2_20261103.md・lose6_3_20261103.md)
    "l1": {"ENOMOTO_MIP_LOSE6_1_STOP": "w", "ENOMOTO_MIP_EARLY_RESTART_JOIN": "1", "ENOMOTO_MIP_LOSE6_1_LIGHT": "0.2"},
    "l2": {"ENOMOTO_MIP_LOSE6_2_PROBE_WAIT": "1", "ENOMOTO_MIP_LOSE6_2_BGPROOF": "1"},
    "l3": {"ENOMOTO_MIP_LOSE6_3_JOINSTOP": "1", "ENOMOTO_MIP_LOSE6_3_CUTRESTART": "hw", "ENOMOTO_T_MIP_LOSE6_3_CUTRESTART_FRAC": "0.3"},
    "l123": {"ENOMOTO_MIP_LOSE6_1_STOP": "w", "ENOMOTO_MIP_EARLY_RESTART_JOIN": "1", "ENOMOTO_MIP_LOSE6_1_LIGHT": "0.2", "ENOMOTO_MIP_LOSE6_2_PROBE_WAIT": "1", "ENOMOTO_MIP_LOSE6_2_BGPROOF": "1", "ENOMOTO_MIP_LOSE6_3_JOINSTOP": "1", "ENOMOTO_MIP_LOSE6_3_CUTRESTART": "hw", "ENOMOTO_T_MIP_LOSE6_3_CUTRESTART_FRAC": "0.3"},
    # uccase12 のメモリ不足の試作 (2026-11-03、bench22。analysis/uccase12_mem1_20261103.md・uccase12_mem2_20261103.md・uccase12_mem3_20261103.md)
    "siblu1r": {"ENOMOTO_MIP_MEM1_SIBLU": "1", "ENOMOTO_MIP_MEM1_SIBLU_ROWS": "1000"},
    "mem2p": {"ENOMOTO_MIP_MEM2": "p"},
    "margin": {"ENOMOTO_MIP_MEM3_TINY_MARGIN": "1"},
    "memall": {"ENOMOTO_MIP_MEM1_SIBLU": "1", "ENOMOTO_MIP_MEM1_SIBLU_ROWS": "1000", "ENOMOTO_MIP_MEM2": "p", "ENOMOTO_MIP_MEM3_TINY_MARGIN": "1"},
    # メモリの試作の既定化 (2026-11-04、analysis/mem_default_20261104.md) の後は "def" が memall と FZ3_REPAIR_BOUND を含む。それより前の既定は "prememall"
    "prememall": {"ENOMOTO_MIP_MEM1_SIBLU": "0", "ENOMOTO_MIP_MEM2": "0", "ENOMOTO_MIP_MEM3_TINY_MARGIN": "0", "ENOMOTO_MIP_FZ3_REPAIR_BOUND": "0"},
    # RSS を見て開いたノードの持ち物を捨てる試作 (2026-11-04、analysis/memcap_a_20261104.md)
    "memcap": {"ENOMOTO_MIP_MEMCAP": "1"},
    # bench24 (2026-11-04): 新既定から 1 つずつ戻す (qiu の遅れの切り分け)
    "nosib": {"ENOMOTO_MIP_MEM1_SIBLU": "0"},
    "nomem2p": {"ENOMOTO_MIP_MEM2": "0"},
    "nomargin": {"ENOMOTO_MIP_MEM3_TINY_MARGIN": "0"},
    # 2026-11-04 の直しの後の既定は MEM3_TINY_MARGIN=m (サブ MIP では以前の余裕)。サブ MIP でも小さい余裕にするのは "marginall"
    "marginall": {"ENOMOTO_MIP_MEM3_TINY_MARGIN": "1"},
    # bench25 (2026-11-04): メモリの試作 3 つ (memcap_a・b・c)、前処理の強制等式 (tree_close2)、解発見スレッドの止め方 (qiu_margin)
    "mem3": {"ENOMOTO_MIP_MEMB": "rcq", "ENOMOTO_MIP_MEMC": "1", "ENOMOTO_MIP_MEMCAP": "1"},
    "forceq": {"ENOMOTO_MIP_TREE2_FORCEQ": "1"},
    "fstop": {"ENOMOTO_MIP_QIU_FIND_STOP_GAP": "1"},
    "all4": {"ENOMOTO_MIP_MEMB": "rcq", "ENOMOTO_MIP_MEMC": "1", "ENOMOTO_MIP_MEMCAP": "1", "ENOMOTO_MIP_TREE2_FORCEQ": "1", "ENOMOTO_MIP_QIU_FIND_STOP_GAP": "1"},
    # mem3・forceq・fstop の既定化 (2026-11-04、analysis/mem3_default_20261104.md) の後は "def" が all4 と同じ。それより前の既定は "preall4"
    "preall4": {"ENOMOTO_MIP_MEMB": "0", "ENOMOTO_MIP_MEMC": "0", "ENOMOTO_MIP_MEMCAP": "0", "ENOMOTO_MIP_TREE2_FORCEQ": "0", "ENOMOTO_MIP_QIU_FIND_STOP_GAP": "0"},
    # 報告の下界の切り上げの既定化 (2026-11-04、analysis/bound_step_20261104.md) の後の既定から戻すのは "prebs"
    "prebs": {"ENOMOTO_MIP_BOUND_STEP": "0"},
    # 2026-11-04 の 600 秒の測り直しで遅くなった 9 問の切り分け: 前の既定 (preall4 + prebs) と 1 つずつ戻した形
    "preall4bs": {"ENOMOTO_MIP_MEMB": "0", "ENOMOTO_MIP_MEMC": "0", "ENOMOTO_MIP_MEMCAP": "0", "ENOMOTO_MIP_TREE2_FORCEQ": "0", "ENOMOTO_MIP_QIU_FIND_STOP_GAP": "0", "ENOMOTO_MIP_BOUND_STEP": "0"},
    "nomem3": {"ENOMOTO_MIP_MEMB": "0", "ENOMOTO_MIP_MEMC": "0", "ENOMOTO_MIP_MEMCAP": "0"},
    "nomemcap": {"ENOMOTO_MIP_MEMCAP": "0"},
    "noforceq": {"ENOMOTO_MIP_TREE2_FORCEQ": "0"},
    "nofstop": {"ENOMOTO_MIP_QIU_FIND_STOP_GAP": "0"},
    "nobs": {"ENOMOTO_MIP_BOUND_STEP": "0"},
    # 2026-11-05: MEMB の q (DSE 重みの 8 ビット丸め) が n2seq36q を遅くするので、q を外した形を比べる
    "membrch": {"ENOMOTO_MIP_MEMB": "rch"},
    "membrc": {"ENOMOTO_MIP_MEMB": "rc"},
    # bench26 (2026-11-05): 探索の脆弱性の試作 (analysis/fragility_20261105.md)
    "rob5": {"ENOMOTO_MIP_ROBUST_ROOTIPM": "1", "ENOMOTO_MIP_ROBUST_STALL": "rw", "ENOMOTO_MIP_ROBUST_LNSFREQ": "1", "ENOMOTO_MIP_ROBUST_IMPORT": "1", "ENOMOTO_MIP_MEMBQ_LNSGROW": "1"},
    "rootipm": {"ENOMOTO_MIP_ROBUST_ROOTIPM": "1"},
    "stallrw": {"ENOMOTO_MIP_ROBUST_STALL": "rw"},
    "lnsx": {"ENOMOTO_MIP_ROBUST_LNSFREQ": "1", "ENOMOTO_MIP_ROBUST_IMPORT": "1", "ENOMOTO_MIP_MEMBQ_LNSGROW": "1"},
    "cfl": {"ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    "halve_cfl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    # bench27 (2026-11-06): 根の LP の失敗対策。ipmx = 内点法 + クロスオーバーの試作一式 (analysis/rootlp_ipm_fail_20261105.md)、
    # combb = 二段解法の尺度の調整 (大きい LP だけ) + 前向きの Harris (安全モード) + 雑音の行の作り直し (analysis/rootlp_simplex_fail_20261105.md の 10 節)
    "ipmx": {"ENOMOTO_T_XO_CLEANUP_HINT": "3", "ENOMOTO_XO_IPM_RETRY_PC": "3", "ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST": "1", "ENOMOTO_IPM_KKT_AUG_RATIO": "4"},
    "combb": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
              "ENOMOTO_SIMPLEX_NOISE_RESET": "0.1", "ENOMOTO_SIMPLEX_NOISE_RESET_W": "e"},
    "ipmcombb": {"ENOMOTO_T_XO_CLEANUP_HINT": "3", "ENOMOTO_XO_IPM_RETRY_PC": "3", "ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST": "1", "ENOMOTO_IPM_KKT_AUG_RATIO": "4",
                 "ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
                 "ENOMOTO_SIMPLEX_NOISE_RESET": "0.1", "ENOMOTO_SIMPLEX_NOISE_RESET_W": "e"},
    # bench28 (2026-11-06): 他セッションの試作で既定の候補。sym = 対称性の検出の直し (HO3_SYMNORM: 行の向きをそろえる、HO6_SYMWORK・SYMREFINE: 手間の上限)、
    # lp = 基底の受け渡し (HO4_BGNB: 裏のカットの LP の非基底の置き場所も渡す、TREE3_RBASIS: 再スタートの後の合わない基底を捨てる)、
    # prr = プロービングの後の前処理のかけ直し (PROBE_REPRESOLVE、衝突解析の直しの後)、all = 全部
    "sym": {"ENOMOTO_MIP_HO3_SYMNORM": "1", "ENOMOTO_MIP_HO6_SYMWORK": "1", "ENOMOTO_MIP_HO6_SYMREFINE": "1"},
    "lp": {"ENOMOTO_MIP_HO4_BGNB": "1", "ENOMOTO_MIP_TREE3_RBASIS": "1"},
    "prr": {"ENOMOTO_MIP_PROBE_REPRESOLVE": "1"},
    "all28": {"ENOMOTO_MIP_HO3_SYMNORM": "1", "ENOMOTO_MIP_HO6_SYMWORK": "1", "ENOMOTO_MIP_HO6_SYMREFINE": "1", "ENOMOTO_MIP_HO4_BGNB": "1",
              "ENOMOTO_MIP_TREE3_RBASIS": "1", "ENOMOTO_MIP_PROBE_REPRESOLVE": "1"},
    # sym・lp の既定化 (2026-11-06、analysis/symlp_default_20261106.md) の後は "def" が sym と lp を含む。それより前の既定は "presymlp"
    "presymlp": {"ENOMOTO_MIP_HO3_SYMNORM": "0", "ENOMOTO_MIP_HO6_SYMWORK": "0", "ENOMOTO_MIP_HO6_SYMREFINE": "0", "ENOMOTO_MIP_HO4_BGNB": "0",
                 "ENOMOTO_MIP_TREE3_RBASIS": "0"},
    # 期限の直し (2026-11-06、analysis/ipmx_harm_20261106.md) の後は "def" が ZH_DEADLINE・POLISH_DEADLINE を含む。それより前の既定は "prezhpol"
    "prezhpol": {"ENOMOTO_MIP_ZH_DEADLINE": "0", "ENOMOTO_MIP_POLISH_DEADLINE": "0"},
    # s4c の既定化 (2026-11-06、analysis/s4c_default_20261106.md) の後は "def" が S4_RINSRATE=0.5・S4_BGZERO=2 を含む。それより前の既定は "pres4c"
    "pres4c": {"ENOMOTO_MIP_S4_RINSRATE": "0", "ENOMOTO_MIP_S4_BGZERO": "0"},
    # 根の LP の尺度の安全網 (2026-11-06、analysis/momentum1_rootlp_20261106.md) の後は "def" が ROOTLP_SCALE_RESCUE を含む。それより前は "prerescue"
    "prerescue": {"ENOMOTO_MIP_ROOTLP_SCALE_RESCUE": "0"},
    # 根の LP を尺度を調整した写しで解く ROOTLP_SCALE の既定化 (2026-11-07、analysis/rootlp_scale_20261106.md) の後は "def" が ROOTLP_SCALE を含む。
    # それより前の既定は "prerootscale"
    "prerootscale": {"ENOMOTO_MIP_ROOTLP_SCALE": "0"},
    # bench29 (2026-11-06): HiGHS より遅い問題の組 S1・S3・S6 の候補 (analysis/highs_slow_20261106/)。s1 = 根の RINS の空振りのやり直し・解発見の解を
    # 内点法の丸めの間も取り込む・クリークの判定の比、s3 = 木の分離の密なカットの判定、s6 = 0 ラウンド目のカットの LP を外さない + 根でサブ MIP を待つ上限 +
    # カットの LP の時間の上限、all29 = 全部
    "s1": {"ENOMOTO_MIP_S1_RINSWIDE": "2", "ENOMOTO_MIP_S1_IPMFIND": "1", "ENOMOTO_T_MIP_CLIQUE_FB_RATIO": "0.1"},
    "s3": {"ENOMOTO_MIP_S3_TSDENSE": "1"},
    "s6": {"ENOMOTO_MIP_HO4_CUT0": "1", "ENOMOTO_MIP_TREE3_JOINCAP": "1", "ENOMOTO_MIP_S6_CUTLP_TCAP": "10"},
    "all29": {"ENOMOTO_MIP_S1_RINSWIDE": "2", "ENOMOTO_MIP_S1_IPMFIND": "1", "ENOMOTO_T_MIP_CLIQUE_FB_RATIO": "0.1", "ENOMOTO_MIP_S3_TSDENSE": "1",
              "ENOMOTO_MIP_HO4_CUT0": "1", "ENOMOTO_MIP_TREE3_JOINCAP": "1", "ENOMOTO_MIP_S6_CUTLP_TCAP": "10"},
    # SYMSIGN・SYMZERO の既定化 (2026-11-06、analysis/symsign_default_20261106.md) の後は "def" が両方を含む。それより前は "presymsign"
    "presymsign": {"ENOMOTO_MIP_SYMSIGN": "0", "ENOMOTO_MIP_SYMZERO": "0"},
    # all1107 (2026-11-07): 今日の既定の候補を全部入れた形 (Mittelmann 600 秒の res4 で既定と比べる)。comb = 二段解法の尺度の調整 (大きい LP だけ) +
    # 前向きの Harris (安全モード) (analysis/combb_600_20261106.md)、gs = 多点分離を最適面が広く停滞したときだけ (analysis/mpsep_20261106.md の 6 節)、
    # MUSHAGG=cover (analysis/mushroom_cuts_20261106.md)、LPSCALE = MIP の LP 全体を尺度を調整した双子で解く (係数の幅 1e4 以上、
    # analysis/mip_lp_scale_20261106.md)、prr (analysis/prr_harm_20261106.md)、SUBFIX=rtc = 根の RENS・木の RINS を目標の固定率で止めるダイブ
    # (analysis/submip_impr_20261106/C.md)、phgb = ノードの LP が重いスレッドだけ費用の摂動 + 前向きの Harris + 保存した置き場所 (analysis/node_lp_20261106.md)、
    # PROBE_BASIS_GUARD=x = 列だけの基底の warm start が長引いたら捨てる (analysis/probe_basis_20261106.md)
    "all1107": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
                "ENOMOTO_MIP_MPSEP": "oqmgs", "ENOMOTO_T_MIP_MPSEP_NV": "1", "ENOMOTO_T_MIP_MPSEP_EXTRA": "5",
                "ENOMOTO_MIP_MUSHAGG": "cover",
                "ENOMOTO_MIP_LPSCALE": "1", "ENOMOTO_T_MIP_LPSCALE_MIN_RANGE": "1e4",
                "ENOMOTO_MIP_PROBE_REPRESOLVE": "1",
                "ENOMOTO_MIP_SUBFIX": "rtc",
                "ENOMOTO_MIP_NODELP_PERTURB": "phg", "ENOMOTO_MIP_BASIS_NB": "t",
                "ENOMOTO_MIP_PROBE_BASIS_GUARD": "x"},
    # cand1107 (2026-11-07): all1107 から LPSCALE (cbs-cta の害、analysis/mitt600_20261103.md の res4 の節) を外し、HNFPRE の bounds
    # (整数列だけの等式の Hermite 標準形で実行不能・固定・上下限の詰め、analysis/hnfpre_20261107.md) を足した形。Mittelmann 600 秒の res5 で既定と比べる
    "cand1107": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
                 "ENOMOTO_MIP_MPSEP": "oqmgs", "ENOMOTO_T_MIP_MPSEP_NV": "1", "ENOMOTO_T_MIP_MPSEP_EXTRA": "5",
                 "ENOMOTO_MIP_MUSHAGG": "cover",
                 "ENOMOTO_MIP_PROBE_REPRESOLVE": "1",
                 "ENOMOTO_MIP_SUBFIX": "rtc",
                 "ENOMOTO_MIP_NODELP_PERTURB": "phg", "ENOMOTO_MIP_BASIS_NB": "t",
                 "ENOMOTO_MIP_PROBE_BASIS_GUARD": "x",
                 "ENOMOTO_MIP_HNFPRE": "bounds"},
    # cand1108 (2026-11-08): cand1107 に 11-08 の改良を入れた形。SUBFIX は保険つきの rtcqf (C.md の 8 節)、PROBE_BASIS_GUARD は x → g (列だけの基底を
    # 前処理の行の順で埋める、probe_basis_20261106.md の 9 節)、CUTLPCAP=ac (根のカットの LP の上限、cutlpcap_20261107.md)、CMIRDELTA=n
    # (cmirdelta_20261107.md)、LPSCALE=h (重い LP だけ双子に移る、mip_lp_scale_20261106.md の 6 節) を足す。Mittelmann 600 秒の res6 で既定と比べる
    "cand1108": {"ENOMOTO_SIMPLEX_SCALE": "1", "ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "10000", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "1e4", "ENOMOTO_SIMPLEX_HARRIS2": "r",
                 "ENOMOTO_MIP_MPSEP": "oqmgs", "ENOMOTO_T_MIP_MPSEP_NV": "1", "ENOMOTO_T_MIP_MPSEP_EXTRA": "5",
                 "ENOMOTO_MIP_MUSHAGG": "cover",
                 "ENOMOTO_MIP_PROBE_REPRESOLVE": "1",
                 "ENOMOTO_MIP_SUBFIX": "rtcqf",
                 "ENOMOTO_MIP_NODELP_PERTURB": "phg", "ENOMOTO_MIP_BASIS_NB": "t",
                 "ENOMOTO_MIP_PROBE_BASIS_GUARD": "g",
                 "ENOMOTO_MIP_HNFPRE": "bounds",
                 "ENOMOTO_MIP_CUTLPCAP": "ac",
                 "ENOMOTO_MIP_CMIRDELTA": "n",
                 "ENOMOTO_MIP_LPSCALE": "h", "ENOMOTO_T_MIP_LPSCALE_MIN_RANGE": "1e4"},
    # res7 (2026-11-08): zerohalf を止めた形 (58 問で既定の zerohalf を止めると SGM -6.2%、analysis/modkvb_20261107.md の 6.3 節)。
    # nozh = 既定 + zerohalf なし、cand1108z = cand1108 + zerohalf なし。res6 (既定・cand1108、同じ種) と並べて 2 x 2 で比べる
    "nozh": {"ENOMOTO_MIP_NO_ZEROHALF": "1"},
    # 根の RENS を目標の固定率で止める (analysis/submip_impr_20261106/A.md の SUBA_RATE の n だけ)
    "suban": {"ENOMOTO_MIP_SUBA_RATE": "n"},
}
CFGS["cand1108z"] = dict(CFGS["cand1108"], ENOMOTO_MIP_NO_ZEROHALF="1")  # res7: cand1108 + zerohalf なし
# res8 (2026-11-09、analysis/mitt600_20261103.md): 厳密な DSE 重みから始める試作 (analysis/nodelp2_20261107.md の 10.2 節) を既定と cand1108z に足す
CFGS["umr"] = {"ENOMOTO_MIP_DSE_INIT": "umr"}
CFGS["cand1108zd"] = dict(CFGS["cand1108z"], ENOMOTO_MIP_DSE_INIT="umr")
# 2026-11-09: cand1108zd を既定にした (analysis/cand1108zd_default_20261109.md)。これからの "def" は cand1108zd を含む。
# precand1108zd = 新しい既定から 20 個を元の振る舞いに戻す形 (切り替えは "0"、tunable は元の既定の値)
CFGS["precand1108zd"] = {k: "0" for k in CFGS["cand1108zd"] if not k.startswith("ENOMOTO_T_")}
CFGS["precand1108zd"].update({"ENOMOTO_T_SIMPLEX_SCALE_MIN_M": "0", "ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE": "0", "ENOMOTO_T_MIP_MPSEP_NV": "2",
                              "ENOMOTO_T_MIP_MPSEP_EXTRA": "10", "ENOMOTO_T_MIP_LPSCALE_MIN_RANGE": "0"})
if __name__ == "__main__":  # bench.py からは設定 (CFGS) だけを読む
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    for rep in range(reps):
        for p in probs:
            for c in cfgs:
                out = D / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
