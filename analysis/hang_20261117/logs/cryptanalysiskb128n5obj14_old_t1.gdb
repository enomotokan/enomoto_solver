=== stack 0 at 601.526494227s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccb0b6b4 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 1 at 601.958862633s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaf0255 in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 2 at 602.418266111s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaf0c81 in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 3 at 602.879738757s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaefb5e in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#2} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccaf0d3d in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 4 at 603.301283648s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccb0b795 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 5 at 603.740419499s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaeeb6c in <std::hash::random::RandomState as core::hash::BuildHasher>::hash_one::<&u64> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccaf0c72 in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 6 at 604.222926451s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaf0cc0 in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 7 at 604.647522362s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaf0c8b in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 8 at 605.096455832s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccb0b702 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
=== stack 9 at 605.479072112s
[New LWP 17526]
[New LWP 26332]
[New LWP 26199]
[New LWP 26198]
[New LWP 26197]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fecc09ff6c0 (LWP 26197) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd610 <thread_status+16>, cond=0x7fecca9bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd638 <thread_status+56>, mutex=0x7fecca9bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fecc01fe6c0 (LWP 26198) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd690 <thread_status+144>, cond=0x7fecca9bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd6b8 <thread_status+184>, mutex=0x7fecca9bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fecbf9fd6c0 (LWP 26199) "python3"):
#0  0x00007feccde98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fecca9bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fecca9bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007feccde9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fecca9bd710 <thread_status+272>, cond=0x7fecca9bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fecca9bd738 <thread_status+312>, mutex=0x7fecca9bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fecc9819ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fecbeffc6c0 (LWP 26332) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd275d56 in <std::thread::thread::Thread>::park () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccd099dd in std::thread::scoped::scope::<_core::mip::feas_fj_lmip_early_salt::{closure#4}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccc9d1aca in _core::mip::feas_fj_lmip_early_salt () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007feccc96f380 in _core::mip::with_presolve_deadline::<core::option::Option<alloc::vec::Vec<f64>>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007feccc71c9b8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::robust_fj_retry () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007feccc79c84b in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccc60b6b2 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccc9c5cf6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007feccc987384 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007feccc974dce in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007feccc9e5b04 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fecccee851e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fecccd59c43 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007feccd23e044 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007feccd23da0b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007feccd23a64a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007feccd240a7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fecbedfb6c0 (LWP 17526) "enomoto-0"):
#0  0x00007fecccaeebb7 in <std::hash::random::RandomState as core::hash::BuildHasher>::hash_one::<&u64> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007feccc85f25f in <hashbrown::raw::RawTable<(usize, f64)>>::reserve_rehash::<hashbrown::map::make_hasher<usize, f64, std::hash::random::RandomState>::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fecccaf03f7 in _core::mip::localmip::chain_move::<_core::mip::localmip::St>::{closure#3} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fecccb0c213 in <_core::mip::localmip::LocalMip>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccd4337d in _core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}::{closure#4} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccd00c31 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd46f26 in <std::thread::lifecycle::spawn_unchecked<_core::mip::feas_fj_lmip_early_salt::{closure#4}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccd287d2f in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007feccde9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007feccdf29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fecce0ca740 (LWP 26158) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007feccd26b8a5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007feccd240bf9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007feccca21758 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fecccda3ded in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fecccdaa51e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fecccd8bc2d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007feccce30680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#9  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#10 0x000000000168f110 in PyEval_EvalCode ()
#11 0x000000000168802f in run_eval_code_obj ()
#12 0x00000000016c42c1 in run_mod ()
#13 0x00000000017fe9ab in _PyRun_File ()
#14 0x00000000017fde42 in _PyRun_SimpleFile ()
#15 0x0000000001800746 in _PyRun_AnyFile ()
#16 0x0000000001800682 in pymain_run_file_obj ()
#17 0x0000000001800578 in pymain_run_file ()
#18 0x000000000178778f in Py_RunMain ()
#19 0x0000000001787454 in pymain_main ()
#20 0x00000000017872b8 in main ()
[Inferior 1 (process 26158) detached]
