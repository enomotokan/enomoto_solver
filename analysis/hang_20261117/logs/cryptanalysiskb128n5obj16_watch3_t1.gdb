=== stack 0 at 600.850943847s
[New LWP 23752]
[New LWP 14359]
[New LWP 14283]
[New LWP 14282]
[New LWP 14281]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fcc05fff6c0 (LWP 14281) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd610 <thread_status+16>, cond=0x7fcc0ffbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd638 <thread_status+56>, mutex=0x7fcc0ffbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcc057fe6c0 (LWP 14282) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd690 <thread_status+144>, cond=0x7fcc0ffbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd6b8 <thread_status+184>, mutex=0x7fcc0ffbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcc04ffd6c0 (LWP 14283) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd710 <thread_status+272>, cond=0x7fcc0ffbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd738 <thread_status+312>, mutex=0x7fcc0ffbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcc046fc6c0 (LWP 14359) "enomoto-0"):
#0  0x00007fcc1230d5e2 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcc12313aa5 in <_core::simplex::lu::FtLu>::solve_into_pair_capture_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc125760be in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc125c3a3a in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc125fef2f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12647d2f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12644f46 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc126561ed in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc11e27a30 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcc11e261ca in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcc11f9395e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcc11e0c472 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcc122475b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcc12209654 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcc121efd4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcc12266dc4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcc120b99ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcc123b20c3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcc12a411c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcc12a40b8b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fcc12a3d7ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fcc12a43bfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcc044fb6c0 (LWP 23752) "memcap"):
#0  0x00007fcc134ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7fcc044fac78, rem=0x7fcc044fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007fcc12a86587 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12365555 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc1239f585 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcc137c9740 (LWP 14259) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcc12a6ea25 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12a43d79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc124bfc28 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc126cde6a in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12193b57 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12199afe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc1216853d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc121deba0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 14259) detached]
=== stack 1 at 601.313374789s
[New LWP 23752]
[New LWP 14359]
[New LWP 14283]
[New LWP 14282]
[New LWP 14281]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fcc05fff6c0 (LWP 14281) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd610 <thread_status+16>, cond=0x7fcc0ffbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd638 <thread_status+56>, mutex=0x7fcc0ffbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcc057fe6c0 (LWP 14282) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd690 <thread_status+144>, cond=0x7fcc0ffbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd6b8 <thread_status+184>, mutex=0x7fcc0ffbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcc04ffd6c0 (LWP 14283) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd710 <thread_status+272>, cond=0x7fcc0ffbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd738 <thread_status+312>, mutex=0x7fcc0ffbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcc046fc6c0 (LWP 14359) "enomoto-0"):
#0  0x00007fcc1230d342 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcc12313aa5 in <_core::simplex::lu::FtLu>::solve_into_pair_capture_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc125760be in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc125c3a3a in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc125fef2f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12647d2f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12644f46 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc126561ed in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc11e27a30 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcc11e261ca in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcc11f9395e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcc11e0c472 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcc122475b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcc12209654 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcc121efd4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcc12266dc4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcc120b99ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcc123b20c3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcc12a411c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcc12a40b8b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fcc12a3d7ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fcc12a43bfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcc044fb6c0 (LWP 23752) "memcap"):
#0  0x00007fcc134ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7fcc044fac78, rem=0x7fcc044fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007fcc12a86587 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12365555 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc1239f585 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcc137c9740 (LWP 14259) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcc12a6ea25 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12a43d79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc124bfc28 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc126cde6a in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12193b57 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12199afe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc1216853d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc121deba0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 14259) detached]
=== stack 2 at 601.706731298s
[New LWP 23752]
[New LWP 14359]
[New LWP 14283]
[New LWP 14282]
[New LWP 14281]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fcc05fff6c0 (LWP 14281) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd610 <thread_status+16>, cond=0x7fcc0ffbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd638 <thread_status+56>, mutex=0x7fcc0ffbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcc057fe6c0 (LWP 14282) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd690 <thread_status+144>, cond=0x7fcc0ffbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd6b8 <thread_status+184>, mutex=0x7fcc0ffbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcc04ffd6c0 (LWP 14283) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd710 <thread_status+272>, cond=0x7fcc0ffbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd738 <thread_status+312>, mutex=0x7fcc0ffbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcc046fc6c0 (LWP 14359) "enomoto-0"):
#0  0x00007fcc122f1ee5 in <_core::simplex::lu::MarkowitzState>::peel_column_singletons () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcc12329c06 in _core::simplex::lu::factorize_flat_markowitz_routed () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc123217b4 in _core::simplex::lu::factorize_routed () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc12321efa in _core::simplex::lu::factorize_reusing () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc12125b05 in _core::simplex::basis_kernel::factorize_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc125e93a7 in _core::simplex::slope_intercept_dual::warm_start_basis () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc125a879e in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc125fef2f in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc12647d2f in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcc12644f46 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcc126561ed in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcc11e27a30 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive_try () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcc11e26217 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::dive () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcc11f9395e in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcc11e0c472 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcc122475b7 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcc12209654 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcc121efd4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcc12266dc4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcc120b99ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fcc123b20c3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fcc12a411c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fcc12a40b8b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fcc12a3d7ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007fcc12a43bfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#26 0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#27 0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcc044fb6c0 (LWP 23752) "memcap"):
#0  0x00007fcc134ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7fcc044fac78, rem=0x7fcc044fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007fcc12a86587 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12365555 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc1239f585 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcc137c9740 (LWP 14259) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcc12a6ea25 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12a43d79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc124bfc28 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc126cde6a in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12193b57 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12199afe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc1216853d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc121deba0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 14259) detached]
=== stack 3 at 602.119961068s
[New LWP 23752]
[New LWP 14359]
[New LWP 14283]
[New LWP 14282]
[New LWP 14281]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 6 (Thread 0x7fcc05fff6c0 (LWP 14281) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd610 <thread_status+16>, cond=0x7fcc0ffbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd638 <thread_status+56>, mutex=0x7fcc0ffbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcc057fe6c0 (LWP 14282) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd690 <thread_status+144>, cond=0x7fcc0ffbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd6b8 <thread_status+184>, mutex=0x7fcc0ffbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcc04ffd6c0 (LWP 14283) "python3"):
#0  0x00007fcc13498e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcc0ffbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcc0ffbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcc1349b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcc0ffbd710 <thread_status+272>, cond=0x7fcc0ffbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcc0ffbd738 <thread_status+312>, mutex=0x7fcc0ffbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcc0ee19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcc046fc6c0 (LWP 14359) "enomoto-0"):
#0  0x00007fcc127986af in mi_free () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcc1260e393 in core::ptr::drop_glue::<_core::mip::problem::MipProblem> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc1266b07d in <alloc::rc::Rc<_core::mip::problem::MipProblem>>::drop_slow () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc12209ba7 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc121efd4e in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12266dc4 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc120b99ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc123b20c3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc12a411c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcc12a40b8b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcc12a3d7ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcc12a43bfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#14 0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcc044fb6c0 (LWP 23752) "memcap"):
#0  0x00007fcc134ecb9f in __GI___clock_nanosleep (clock_id=1, flags=0, req=0x7fcc044fac78, rem=0x7fcc044fac78) at ../sysdeps/unix/sysv/linux/clock_nanosleep.c:78
#1  0x00007fcc12a86587 in std::thread::functions::sleep () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12365555 in std::sys::backtrace::__rust_begin_short_backtrace::<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc1239f585 in <std::thread::lifecycle::spawn_unchecked<_core::mip::solver::memcap_start::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc12a8aeaf in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc1349cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#6  0x00007fcc13529e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcc137c9740 (LWP 14259) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcc12a6ea25 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcc12a43d79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcc124bfc28 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcc126cde6a in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcc12193b57 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcc12199afe in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcc1216853d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcc121deba0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 14259) detached]
=== stack 4 at 602.582833371s
ptrace: No such process.
