=== stack 0 at 61.596580771s
[New LWP 18990]
[New LWP 18989]
[New LWP 18988]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x0000000001644b8c in gc_collect_main ()

Thread 4 (Thread 0x7f22615ff6c0 (LWP 18988) "python3"):
#0  0x00007f226e498e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f226b5bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f226b5bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f226b5bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f226e49b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f226b5bd610 <thread_status+16>, cond=0x7f226b5bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f226b5bd638 <thread_status+56>, mutex=0x7f226b5bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f226a419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f226e49cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f226e529ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f2260dfe6c0 (LWP 18989) "python3"):
#0  0x00007f226e498e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f226b5bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f226b5bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f226b5bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f226e49b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f226b5bd690 <thread_status+144>, cond=0x7f226b5bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f226b5bd6b8 <thread_status+184>, mutex=0x7f226b5bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f226a419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f226e49cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f226e529ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f22605fd6c0 (LWP 18990) "python3"):
#0  0x00007f226e498e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f226b5bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f226b5bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f226b5bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f226e49b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f226b5bd710 <thread_status+272>, cond=0x7f226b5bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f226b5bd738 <thread_status+312>, mutex=0x7f226b5bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f226a419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f226e49cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f226e529ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f226e76d740 (LWP 18982) "python3"):
#0  0x0000000001644b8c in gc_collect_main ()
#1  0x000000000179063f in finalize_modules ()
#2  0x000000000178fa89 in _Py_Finalize ()
#3  0x0000000001787706 in Py_RunMain ()
#4  0x0000000001787454 in pymain_main ()
#5  0x00000000017872b8 in main ()
[Inferior 1 (process 18982) detached]
