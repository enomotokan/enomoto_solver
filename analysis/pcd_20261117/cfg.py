"""集約の 2 つの試作の確かめ (analysis/prbsubst_20261117・analysis/dualfree_20261117)。既定は変えない。
- def = 既定 (環境変数なし)
- pc = ENOMOTO_AGG_PIVOT_COL=1 (集約のピボットの比を列の側でも判定する)
- pcd = pc + ENOMOTO_AGG_DUALFREE_INEQ=1 (双対で陰に自由な不等式行を等式に上げる)"""
PC = {"ENOMOTO_AGG_PIVOT_COL": "1"}
PCD = PC | {"ENOMOTO_AGG_DUALFREE_INEQ": "1"}
CFGS = {"def": {}, "pc": PC, "pcd": PCD}
