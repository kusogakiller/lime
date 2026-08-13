# Phase 2 Metrics — Lime IR vs Clang IR (static counts, evidence)

Counts are STATIC instruction occurrences in the emitted LLVM IR (pre-LLVM-opt for Lime; -O2/-O3 for Clang).
Runtime calls = calls to @runtime_* helpers. rt=runtime calls, alc=alloca, ld=load, st=store, br=branch, gep=getelementptr.

bench          ver     lines  call   rt  alc    ld    st  concat  slice  listadd  listget
-----------------------------------------------------------------------------------------
string_access  lime      541     6    2    5    12     9       0      0        0        0
               cO2       130    18    0    1     2     1       0      0        0        0
               cO3       130    18    0    1     2     1       0      0        0        0

string_concat  lime      513     4    1    2     4     4       0      0        0        0
               cO2       118    16    0    1     2     2       0      0        0        0
               cO3       118    16    0    1     2     2       0      0        0        0

int_loop       lime      510     2    0    2     5     4       0      0        0        0
               cO2       100     8    0    1     2     0       0      0        0        0
               cO3       100     8    0    1     2     0       0      0        0        0

func_call      lime      548     6    0    6    11     8       0      0        0        0
               cO2       115     9    0    1     2     0       0      0        0        0
               cO3       115     9    0    1     2     0       0      0        0        0

struct_ops     lime      567     5    0    9    18    11       0      0        0        0
               cO2        77     8    0    1     2     0       0      0        0        0
               cO3        77     8    0    1     2     0       0      0        0        0

list_iter      lime      542     3    1    6    13     9       0      0        0        0
               cO2       163    11    0    1    12     2       0      0        0        0
               cO3       163    11    0    1    12     2       0      0        0        0

list_push      lime      522     4    1    4     8     5       0      0        0        0
               cO2       115    11    0    1     3     2       0      0        0        0
               cO3       115    11    0    1     3     2       0      0        0        0

map_ops        lime      585     5    2   11    24    16       0      0        0        0
               cO2       188    13    0    1    10     7       0      0        0        0
               cO3       189    13    0    1    10     7       0      0        0        0

set_ops        lime      613     4    1   11    28    18       0      0        0        0
               cO2       190    11    0    1     8     3       0      0        0        0
               cO3       191    11    0    1     8     3       0      0        0        0

control_flow   lime      547     2    0    2    13     7       0      0        0        0
               cO2       120     8    0    1     2     0       0      0        0        0
               cO3       120     8    0    1     2     0       0      0        0        0






## Frozen baseline median (ms) + ratio (Lime / Clang O3) for reference

bench                Lime    ClangO2    ClangO3    L/O3
----------------------------------------------------------
string_access       84.78      22.28      20.45   4.147
string_concat      233.38     143.30     146.34   1.595
int_loop            92.84      88.63      88.10   1.054
func_call           29.64      25.92      25.83   1.147
struct_ops          11.81       7.61       7.06   1.671
list_iter           11.86       7.60       7.41   1.600
list_push           11.94       7.23       7.49   1.595
map_ops             16.43      11.07      11.63   1.412
set_ops            188.71     100.84     104.99   1.797
control_flow        84.29      69.36      69.95   1.205
recursion_tree      11.28       7.00       7.04   1.601
memory_alloc        11.08       7.43       7.17   1.547
algo_sieve          11.75       7.40       7.55   1.557
algo_sort           12.50       7.98       7.42   1.684