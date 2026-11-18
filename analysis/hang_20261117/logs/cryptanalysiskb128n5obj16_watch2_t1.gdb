=== stack 0 at 600.505720724s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c4716bd9 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c49cb5c4 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4225eca in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#22 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 1 at 601.134277445s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c470d042 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c47137a5 in <_core::simplex::lu::FtLu>::solve_into_pair_capture_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c498a8ee in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c49d826a in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4225eca in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 2 at 601.578876024s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  __memcpy_avx512_unaligned_erms () at ../sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S:660
#1  0x00007f01c58ad148 in __GI_memcpy (__len=<optimized out>, __src=0x7f01894b35b0, __dest=0x7f0188f5e930) at ../string/bits/string_fortified.h:29
#2  _int_realloc (av=av@entry=0x7f01ac000030, oldp=oldp@entry=0x7f01894b35a0, oldsize=oldsize@entry=1572880, nb=3145744) at ./malloc/malloc.c:4996
#3  0x00007f01c58ae2c6 in __GI___libc_realloc (oldmem=0x7f01894b35b0, bytes=3145728) at ./malloc/malloc.c:3517
#4  0x00007f01c496fb1e in <alloc::raw_vec::RawVecInner>::finish_grow () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c496e97a in <alloc::raw_vec::RawVec<(u64, u8, u64, bool)>>::grow_one () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c46f1b22 in <_core::simplex::lu::MarkowitzState>::peel_column_singletons () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4729906 in _core::simplex::lu::factorize_flat_markowitz_routed () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c47214b4 in _core::simplex::lu::factorize_routed () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4721bfa in _core::simplex::lu::factorize_reusing () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c4525805 in _core::simplex::basis_kernel::factorize_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c49fdbd7 in _core::simplex::slope_intercept_dual::warm_start_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c49bcfce in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#26 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#27 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#28 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#29 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 3 at 601.974921606s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c480adeb in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#23 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 4 at 602.406319120s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c4b1877b in core::slice::sort::shared::smallsort::small_sort_network::<u64, <u64 as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4718346 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#26 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 5 at 602.875370828s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c4714b75 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 6 at 603.348935657s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c44a19fd in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c4718346 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#25 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 7 at 603.734747550s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c44a1a01 in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4718346 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#26 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#27 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#28 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 8 at 604.156947488s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c47168b9 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
=== stack 9 at 604.614838294s
[New LWP 4096]
[New LWP 28853]
[New LWP 28793]
[New LWP 28792]
[New LWP 28791]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7f01b83ff6c0 (LWP 28791) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd610 <thread_status+16>, cond=0x7f01c23bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd638 <thread_status+56>, mutex=0x7f01c23bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f01b7bfe6c0 (LWP 28792) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd690 <thread_status+144>, cond=0x7f01c23bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd6b8 <thread_status+184>, mutex=0x7f01c23bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f01b73fd6c0 (LWP 28793) "python3"):
#0  0x00007f01c5898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f01c23bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f01c23bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f01c589b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f01c23bd710 <thread_status+272>, cond=0x7f01c23bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f01c23bd738 <thread_status+312>, mutex=0x7f01c23bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f01c1219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f01b6afc6c0 (LWP 28853) "enomoto-0"):
#0  0x00007f01c4b18937 in core::slice::sort::shared::smallsort::small_sort_network::<u64, <u64 as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c44a1aca in core::slice::sort::unstable::quicksort::quicksort::<usize, <usize as core::cmp::PartialOrd>::lt> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4718346 in <_core::simplex::lu::FtLu>::solve_transpose_unit_work_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c480ad9d in _core::simplex::dse_exact_row_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c47faa45 in <_core::simplex::DseState>::from_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c49c19f3 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f01c4a1375f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f01c4a5c53f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f01c4a59756 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f01c4a6a9fd in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f01c4227730 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f01c4225f17 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f01c439365e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f01c420c172 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f01c46472b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f01c4609354 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f01c45efa4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f01c4666ac4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f01c44b96ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f01c47b1dc3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007f01c4e40744 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007f01c4e4010b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007f01c4e3cd4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#26 0x00007f01c4e4317d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#27 0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#28 0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#29 0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f01b68fb6c0 (LWP 4096) "memcap"):
#0  0x00007f01c58ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7f01b68fac78, rem=0x7f01b68fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007f01c4e85b07 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4765255 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c479f285 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c4e8a42f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c589cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007f01c5929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f01c5bc9740 (LWP 28745) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f01c4e6dfa5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f01c4e432f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f01c4951e88 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f01c47d83da in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f01c4593857 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f01c45997fe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f01c456823d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f01c45de8a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 28745) detached]
