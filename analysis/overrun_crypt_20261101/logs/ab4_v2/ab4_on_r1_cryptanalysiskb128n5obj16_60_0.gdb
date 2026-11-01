=== stack 0 at 61.604741069s
[New LWP 3306]
[New LWP 3305]
[New LWP 3304]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x00007ff3808a0f06 in mi_free () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so

Thread 4 (Thread 0x7ff3745ff6c0 (LWP 3304) "python3"):
#0  0x00007ff381698e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7ff37e5bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7ff37e5bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7ff37e5bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007ff38169b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7ff37e5bd610 <thread_status+16>, cond=0x7ff37e5bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7ff37e5bd638 <thread_status+56>, mutex=0x7ff37e5bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007ff37d419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007ff38169cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007ff381729ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7ff373dfe6c0 (LWP 3305) "python3"):
#0  0x00007ff381698e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7ff37e5bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7ff37e5bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7ff37e5bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007ff38169b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7ff37e5bd690 <thread_status+144>, cond=0x7ff37e5bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7ff37e5bd6b8 <thread_status+184>, mutex=0x7ff37e5bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007ff37d419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007ff38169cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007ff381729ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7ff3735fd6c0 (LWP 3306) "python3"):
#0  0x00007ff381698e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7ff37e5bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7ff37e5bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7ff37e5bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007ff38169b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7ff37e5bd710 <thread_status+272>, cond=0x7ff37e5bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7ff37e5bd738 <thread_status+312>, mutex=0x7ff37e5bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007ff37d419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007ff38169cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007ff381729ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7ff3818f0740 (LWP 3298) "python3"):
#0  0x00007ff3808a0f06 in mi_free () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007ff3806c31a2 in core::ptr::drop_glue::<_core::model::PyModel> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007ff3806ec891 in <pyo3::pycell::impl_::PyClassObject<_core::model::PyModel> as pyo3::pycell::impl_::PyClassObjectLayout<_core::model::PyModel>>::tp_dealloc () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007ff3806ce921 in pyo3::impl_::trampoline::trampoline_unraisable::<pyo3::impl_::trampoline::dealloc::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007ff3806ce9e0 in pyo3::impl_::pyclass::tp_dealloc::<_core::model::PyModel> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x0000000001611a6f in _PyEval_EvalFrameDefault ()
#6  0x000000000168f110 in PyEval_EvalCode ()
#7  0x000000000168802f in run_eval_code_obj ()
#8  0x00000000016c42c1 in run_mod ()
#9  0x00000000017fe9ab in _PyRun_File ()
#10 0x00000000017fde42 in _PyRun_SimpleFile ()
#11 0x0000000001800746 in _PyRun_AnyFile ()
#12 0x0000000001800682 in pymain_run_file_obj ()
#13 0x0000000001800578 in pymain_run_file ()
#14 0x000000000178778f in Py_RunMain ()
#15 0x0000000001787454 in pymain_main ()
#16 0x00000000017872b8 in main ()
[Inferior 1 (process 3298) detached]
