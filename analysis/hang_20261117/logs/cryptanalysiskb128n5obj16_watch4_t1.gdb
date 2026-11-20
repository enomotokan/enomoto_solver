=== stack 0 at 600.805746852s
[New LWP 14464]
[New LWP 1685]
[New LWP 1624]
[New LWP 1623]
[New LWP 1622]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f1b14fff6c0 (LWP 1622) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd610 <thread_status+16>, cond=0x7f1b1efbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd638 <thread_status+56>, mutex=0x7f1b1efbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f1b147fe6c0 (LWP 1623) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd690 <thread_status+144>, cond=0x7f1b1efbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd6b8 <thread_status+184>, mutex=0x7f1b1efbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f1b13ffd6c0 (LWP 1624) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd710 <thread_status+272>, cond=0x7f1b1efbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd738 <thread_status+312>, mutex=0x7f1b1efbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f1b135fc6c0 (LWP 1685) "enomoto-0"):
#0  0x00007f1b213bc62c in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f1b213ff1df in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b2144800f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1b21445226 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1b214564cd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1b20c27d18 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1b20c2641a in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f1b20d93c0e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f1b20c0c6c2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f1b21047867 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f1b21009904 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f1b20fefffe in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f1b21067074 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f1b20eb9c5e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f1b211b2373 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f1b21841484 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f1b21840e4b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f1b2183da8a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f1b21843ebd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f1b2188b16f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f1b133fb6c0 (LWP 14464) "memcap"):
#0  0x00007f1b224ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f1b133fac78, rem=0x7f1b133fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f1b21886847 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b21165805 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1b2119f835 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1b2188b16f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f1b226ee740 (LWP 1607) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f1b2186ece5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b21844039 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1b212bfed8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1b214ce11a in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1b20f93e07 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1b20f99dae in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f1b20f687ed in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f1b20fdee50 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#10 0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#11 0x000000000168f110 in PyEval_EvalCode ()
#12 0x000000000168802f in run_eval_code_obj ()
#13 0x00000000016c42c1 in run_mod ()
#14 0x00000000017fe9ab in _PyRun_File ()
#15 0x00000000017fde42 in _PyRun_SimpleFile ()
#16 0x0000000001800746 in _PyRun_AnyFile ()
#17 0x0000000001800682 in pymain_run_file_obj ()
#18 0x0000000001800578 in pymain_run_file ()
#19 0x000000000178778f in Py_RunMain ()
#20 0x0000000001787454 in pymain_main ()
#21 0x00000000017872b8 in main ()
[Inferior 1 (process 1607) detached]
=== stack 1 at 601.268867901s
[New LWP 14464]
[New LWP 1685]
[New LWP 1624]
[New LWP 1623]
[New LWP 1622]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x00007f1b20fe046b in <alloc::collections::btree::map::BTreeMap<usize, usize> as core::ops::drop::Drop>::drop () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so

Thread 6 (Thread 0x7f1b14fff6c0 (LWP 1622) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd610 <thread_status+16>, cond=0x7f1b1efbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd638 <thread_status+56>, mutex=0x7f1b1efbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f1b147fe6c0 (LWP 1623) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd690 <thread_status+144>, cond=0x7f1b1efbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd6b8 <thread_status+184>, mutex=0x7f1b1efbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f1b13ffd6c0 (LWP 1624) "python3"):
#0  0x00007f1b22498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1b1efbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1b1efbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1b1efbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1b2249b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1b1efbd710 <thread_status+272>, cond=0x7f1b1efbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1b1efbd738 <thread_status+312>, mutex=0x7f1b1efbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1b1de19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f1b135fc6c0 (LWP 1685) "enomoto-0"):
#0  0x00007f1b21598963 in mi_free () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f1b211734c3 in std::sys::thread_local::native::eager::destroy::<core::cell::RefCell<alloc::vec::Vec<alloc::vec::Vec<(usize, f64)>>>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b2244772f in __GI___call_tls_dtors () at ./stdlib/cxa_thread_atexit_impl.c:156
#3  0x00007f1b2249c94b in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:452
#4  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f1b133fb6c0 (LWP 14464) "memcap"):
#0  0x00007f1b224ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f1b133fac78, rem=0x7f1b133fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f1b21886847 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b21165805 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1b2119f835 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1b2188b16f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1b2249cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f1b22529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f1b226ee740 (LWP 1607) "python3"):
#0  0x00007f1b20fe046b in <alloc::collections::btree::map::BTreeMap<usize, usize> as core::ops::drop::Drop>::drop () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f1b20fe85d2 in core::ptr::drop_glue::<_core::model::PyModel> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1b2107ded1 in <pyo3::pycell::impl_::PyClassObject<_core::model::PyModel> as pyo3::pycell::impl_::PyClassObjectLayout<_core::model::PyModel>>::tp_dealloc () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1b20eba041 in pyo3::impl_::trampoline::trampoline_unraisable::<pyo3::impl_::trampoline::dealloc::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1b20eba100 in pyo3::impl_::pyclass::tp_dealloc::<_core::model::PyModel> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 1607) detached]
=== stack 2 at 601.700591190s
ptrace: No such process.
