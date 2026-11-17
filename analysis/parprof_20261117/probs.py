"""並列の効きの内訳 (treeclose 9.69 節) の対象。P = 58 問のうち既定 (cand1118) で 1 スレッドか 4 スレッドのどちらかが解け、どちらかが 10 秒以上の 23 問
(cand1118_default_20261116 の種 1 の def_t1・def から)。M = treeclose の重い 4 問。"""
P = ['10teams', 'binkar10_1', 'eil33-2', 'h80x6320d', 'hypothyroid-k1', 'markshare_4_0', 'mas76', 'mc11', 'misc07', 'neos-3083819-nubu',
     'neos-860300', 'neos-911970', 'neos5', 'noswot', 'ns1116954', 'nw04', 'physiciansched6-2', 'pk1', 'qiu', 'roll3000', 'rout', 'swath1', 'timtab1']
M = ['neos-3627168-kasai', 'csched008', 'sct2', 'tbfp-network']
THREADS = [1, 2, 4]
SEEDS = [1, 2]
TL = {"P": 120, "M": 600}
