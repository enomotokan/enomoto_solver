O() { python3 ours.py --log "$@" > /dev/null 2>&1; }
O dano3_5 t1rens10 --threads 1 -- ENOMOTO_MIP_T1RENS=0.1 & O dano3_5 t1rens10 --threads 1 --seed 2 -- ENOMOTO_MIP_T1RENS=0.1 & O dano3_5 t1rens20 --threads 1 -- ENOMOTO_MIP_T1RENS=0.2 & O dano3_5 t1rens20 --threads 1 --seed 2 -- ENOMOTO_MIP_T1RENS=0.2 & wait
O seymour cutz -- ENOMOTO_MIP_CUTMIX=z
echo done > chain2.done
