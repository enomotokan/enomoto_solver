cd "$(dirname "$0")"
for i in b c d; do ./run1.sh t4_s1_$i 4 1; done
for s in 1 2 3 4 5; do ./run1.sh t1_s$s 1 $s; done
for s in 2 3 4 5 6 7 8 9 10; do ./run1.sh t4_s$s 4 $s; done
