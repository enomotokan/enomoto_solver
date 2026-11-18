=== stack 0 at 23.222259092s
[New LWP 25375]
[New LWP 25295]
[New LWP 25294]
[New LWP 25293]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 5 (Thread 0x7f67685ff6c0 (LWP 25293) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd610 <thread_status+16>, cond=0x7f67725bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd638 <thread_status+56>, mutex=0x7f67725bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f6767dfe6c0 (LWP 25294) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd690 <thread_status+144>, cond=0x7f67725bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd6b8 <thread_status+184>, mutex=0x7f67725bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f67675fd6c0 (LWP 25295) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd710 <thread_status+272>, cond=0x7f67725bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd738 <thread_status+312>, mutex=0x7f67725bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f6766afc6c0 (LWP 25375) "enomoto-0"):
#0  0x00007f67748ee292 in <_core::simplex::lu::FtLu>::apply_r_sparse () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f67748f8940 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f677490573f in <_core::simplex::lu::FtLu>::solve_sparse_into_pair_capture_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f677491e6c9 in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f677496cae5 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f67749b12e5 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f6774cd1062 in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f6774ccec26 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f6774ce0bd5 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f6774564d15 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f677440b782 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f6774c53e56 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f6774c15364 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f6774c02cae in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f6774c73f14 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f6774d5b2ce in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f6774b34153 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f677503c304 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f677503bccb in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f677503890a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f677503ed3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007f6775085fef in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#23 0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f6775dff740 (LWP 25278) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f6775069b65 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f677503eeb9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f67749db1b8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f67747e35c6 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f67747e918e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f67747cb1ed in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f6774872270 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 25278) detached]
=== stack 1 at 23.597153976s
[New LWP 25375]
[New LWP 25295]
[New LWP 25294]
[New LWP 25293]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 5 (Thread 0x7f67685ff6c0 (LWP 25293) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd610 <thread_status+16>, cond=0x7f67725bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd638 <thread_status+56>, mutex=0x7f67725bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f6767dfe6c0 (LWP 25294) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd690 <thread_status+144>, cond=0x7f67725bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd6b8 <thread_status+184>, mutex=0x7f67725bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f67675fd6c0 (LWP 25295) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd710 <thread_status+272>, cond=0x7f67725bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd738 <thread_status+312>, mutex=0x7f67725bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f6766afc6c0 (LWP 25375) "enomoto-0"):
#0  0x00007f6774d91fac in _mi_page_retire () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f6774bfc0f3 in core::ptr::drop_glue::<_core::mip::problem::MipProblem> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f6774c541f5 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f6774c15364 in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f6774c02cae in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f6774c73f14 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f6774d5b2ce in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f6774b34153 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f677503c304 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f677503bccb in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f677503890a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f677503ed3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f6775085fef in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#14 0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f6775dff740 (LWP 25278) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f6775069b65 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f677503eeb9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f67749db1b8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f67747e35c6 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f67747e918e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f67747cb1ed in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f6774872270 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 25278) detached]
=== stack 2 at 24.058497599s
[New LWP 25295]
[New LWP 25294]
[New LWP 25293]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x00007f6775b2534b in __GI_madvise () at ../sysdeps/unix/syscall-template.S:117

warning: 117	../sysdeps/unix/syscall-template.S: No such file or directory

Thread 4 (Thread 0x7f67685ff6c0 (LWP 25293) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd610 <thread_status+16>, cond=0x7f67725bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd638 <thread_status+56>, mutex=0x7f67725bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f6767dfe6c0 (LWP 25294) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd690 <thread_status+144>, cond=0x7f67725bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd6b8 <thread_status+184>, mutex=0x7f67725bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f67675fd6c0 (LWP 25295) "python3"):
#0  0x00007f6775a98e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f67725bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f67725bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f6775a9b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f67725bd710 <thread_status+272>, cond=0x7f67725bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f67725bd738 <thread_status+312>, mutex=0x7f67725bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f6771419ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f6775a9cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f6775b29e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f6775dff740 (LWP 25278) "python3"):
#0  0x00007f6775b2534b in __GI_madvise () at ../sysdeps/unix/syscall-template.S:117
#1  0x00007f6774d89062 in _mi_prim_decommit () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f6774d8e39f in _mi_os_purge_ex () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f6774d8e489 in mi_arena_purge.isra () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f6774d8e59e in mi_arena_try_purge_visitor () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f6774d8527c in _mi_bitmap_forall_setc_ranges () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f6774d90679 in _mi_arenas_collect () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f6774d93468 in mi_theap_collect_ex () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f6774d969e2 in mi_process_done () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f6775f170f2 in _dl_call_fini (closure_map=closure_map@entry=0x281b4730) at ./elf/dl-call_fini.c:43
#10 0x00007f6775f1b578 in _dl_fini () at ./elf/dl-fini.c:114
#11 0x00007f6775a47a76 in __run_exit_handlers (status=0, listp=<optimized out>, run_list_atexit=run_list_atexit@entry=true, run_dtors=run_dtors@entry=true) at ./stdlib/exit.c:108
#12 0x00007f6775a47bbe in __GI_exit (status=<optimized out>) at ./stdlib/exit.c:138
#13 0x00007f6775a2a1d1 in __libc_start_call_main (main=main@entry=0x178728c <main>, argc=argc@entry=8, argv=argv@entry=0x7fff81f264a8) at ../sysdeps/nptl/libc_start_call_main.h:74
#14 0x00007f6775a2a28b in __libc_start_main_impl (main=0x178728c <main>, argc=8, argv=0x7fff81f264a8, init=<optimized out>, fini=<optimized out>, rtld_fini=<optimized out>, stack_end=0x7fff81f26498) at ../csu/libc-start.c:360
#15 0x00000000017d7a29 in _start ()
[Inferior 1 (process 25278) detached]
