=== stack 0 at 62.514857489s
[New LWP 12097]
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb7987a7541 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb7987ad88a in <_core::simplex::lu::FtLu>::solve_into_pair_capture_tracked () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798686d4e in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7986cb9db in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7986f7bcc in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb798933400 in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb78a2fa6c0 (LWP 12097) "enomoto-0"):
#0  0x00007fb798a8aacf in <_core::mip::domain::Domain>::propagate () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798ac1cba in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798aa9c6f in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798a8f4a9 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798903c0e in _core::cancel::with_halt::<core::option::Option<(_core::mip::problem::MipProblem, alloc::vec::Vec<_core::presolve::PostsolveStep>, core::option::Option<_core::presolve::scaling::Scaling>)>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}::{closure#4}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7988339a3 in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79886adee in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 1 at 63.075720289s
[New LWP 12097]
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb7986d370b in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb7986f7bcc in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#17 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb79893341c in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb78a2fa6c0 (LWP 12097) "enomoto-0"):
#0  0x00007fb798a857d7 in <_core::mip::domain::Domain>::update_activity () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798a84720 in <_core::mip::domain::Domain>::backtrack_to () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798ac1f19 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798aa9c6f in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798a8f4a9 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798903c0e in _core::cancel::with_halt::<core::option::Option<(_core::mip::problem::MipProblem, alloc::vec::Vec<_core::presolve::PostsolveStep>, core::option::Option<_core::presolve::scaling::Scaling>)>, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}::{closure#4}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb7988339a3 in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79886adee in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#10 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 2 at 63.727511909s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb7986ce729 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb7986f7bcc in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#17 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb79893341c in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 3 at 64.318420847s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  __memcpy_evex_unaligned_erms () at ../sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S:266
#1  0x00007fb79878f954 in <_core::simplex::lu::MarkowitzState>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb7987c2175 in _core::simplex::lu::factorize_flat_markowitz_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7987ba164 in _core::simplex::lu::factorize_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7987ba8ca in _core::simplex::lu::factorize_reusing () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7988db4b5 in _core::simplex::basis_kernel::factorize_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb7986e3e23 in _core::simplex::slope_intercept_dual::warm_start_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb7986b3221 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb7986f7de1 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#24 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb798933407 in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 4 at 64.899493977s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb79878eefd in <_core::simplex::lu::MarkowitzState>::peel_column_singletons () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb7987c23c5 in _core::simplex::lu::factorize_flat_markowitz_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb7987ba164 in _core::simplex::lu::factorize_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7987ba8ca in _core::simplex::lu::factorize_reusing () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988db4b5 in _core::simplex::basis_kernel::factorize_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7986e3e23 in _core::simplex::slope_intercept_dual::warm_start_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb7986b3221 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb7986f7de1 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#23 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb798933404 in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 5 at 65.494616872s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb7987920a0 in <_core::simplex::lu::MarkowitzState>::eliminate () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb7987c2b0f in _core::simplex::lu::factorize_flat_markowitz_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb7987ba164 in _core::simplex::lu::factorize_routed () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7987ba8ca in _core::simplex::lu::factorize_reusing () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988db4b5 in _core::simplex::basis_kernel::factorize_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb7986e3e23 in _core::simplex::slope_intercept_dual::warm_start_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb7986b3221 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb7986f7de1 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#23 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb798933400 in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 6 at 66.022927853s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9429]
[New LWP 9428]
[New LWP 9427]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb798ada766 in mi_page_free_list_extend () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798ae28f4 in mi_page_extend_free.isra () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798aedad8 in mi_page_queue_find_free_ex () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798aedd15 in _mi_malloc_generic () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798aee228 in mi_theap_malloc_zero_aligned_at_generic () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798a5ef77 in <core::iter::adapters::map::Map<core::iter::adapters::enumerate::Enumerate<core::slice::iter::Iter<alloc::vec::Vec<(usize, f64)>>>, _core::simplex::lu::factorize_diagonal::{closure#1}> as core::iter::traits::iterator::Iterator>::fold::<(), core::iter::traits::iterator::Iterator::for_each::call<alloc::vec::Vec<(usize, f64)>, <alloc::vec::Vec<alloc::vec::Vec<(usize, f64)>>>::extend_trusted<core::iter::adapters::map::Map<core::iter::adapters::enumerate::Enumerate<core::slice::iter::Iter<alloc::vec::Vec<(usize, f64)>>>, _core::simplex::lu::factorize_diagonal::{closure#1}>>::{closure#0}>::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb7985d8190 in <alloc::vec::Vec<alloc::vec::Vec<(usize, f64)>> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<alloc::vec::Vec<(usize, f64)>, core::iter::adapters::map::Map<core::iter::adapters::enumerate::Enumerate<core::slice::iter::Iter<alloc::vec::Vec<(usize, f64)>>>, _core::simplex::lu::factorize_diagonal::{closure#1}>>>::from_iter () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb7987bfdda in _core::simplex::lu::factorize_diagonal () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb7988db40e in _core::simplex::basis_kernel::factorize_basis () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb7986b3cfd in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986f7de1 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb798612fec in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb79861c242 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb79856aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb7984b5fc0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798ab4190 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798a904a0 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798acacd1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798900f44 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798873ec3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#26 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78aafb6c0 (LWP 9427) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78a6fc6c0 (LWP 9428) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78a4fb6c0 (LWP 9429) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb798933400 in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988fb0e8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798914b1f in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79890bd9d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798925300 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x000000000165f31f in method_vectorcall_FASTCALL_KEYWORDS ()
#8  0x0000000001614f6f in _PyEval_EvalFrameDefault ()
#9  0x000000000168f110 in PyEval_EvalCode ()
#10 0x000000000168802f in run_eval_code_obj ()
#11 0x00000000016c42c1 in run_mod ()
#12 0x00000000017fe9ab in _PyRun_File ()
#13 0x00000000017fde42 in _PyRun_SimpleFile ()
#14 0x0000000001800746 in _PyRun_AnyFile ()
#15 0x0000000001800682 in pymain_run_file_obj ()
#16 0x0000000001800578 in pymain_run_file ()
#17 0x000000000178778f in Py_RunMain ()
#18 0x0000000001787454 in pymain_main ()
#19 0x00000000017872b8 in main ()
[Inferior 1 (process 9267) detached]
=== stack 7 at 66.624804731s
[New LWP 9519]
[New LWP 9518]
[New LWP 9517]
[New LWP 9516]
[New LWP 9426]
[New LWP 9305]
[New LWP 9304]
[New LWP 9303]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x00007fb7988153ae in <alloc::collections::btree::map::BTreeMap<usize, usize> as core::ops::drop::Drop>::drop () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so

Thread 9 (Thread 0x7fb78c7ff6c0 (LWP 9303) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd610 <thread_status+16>, cond=0x7fb7967bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd638 <thread_status+56>, mutex=0x7fb7967bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb78bffe6c0 (LWP 9304) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd690 <thread_status+144>, cond=0x7fb7967bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd6b8 <thread_status+184>, mutex=0x7fb7967bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb78b7fd6c0 (LWP 9305) "python3"):
#0  0x00007fb799898e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb7967bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7967bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb79989b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb7967bd710 <thread_status+272>, cond=0x7fb7967bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb7967bd738 <thread_status+312>, mutex=0x7fb7967bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb795619ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb78acfc6c0 (LWP 9426) "enomoto-0"):
#0  0x00007fb79992601b in __GI_munmap () at ../sysdeps/unix/syscall-template.S:117
#1  0x00007fb7998aad86 in heap_trim (pad=131072, heap=<optimized out>) at ./malloc/arena.c:567
#2  _int_free_maybe_consolidate (av=0x7fb780000030, size=<optimized out>) at ./malloc/malloc.c:4795
#3  0x00007fb7998ab51a in _int_free (av=0x7fb780000030, p=<optimized out>, have_lock=<optimized out>) at ./malloc/malloc.c:4646
#4  0x00007fb7998adeae in __GI___libc_free (mem=0x7fb7314345d0) at ./malloc/malloc.c:3398
#5  0x00007fb79890c952 in std::sys::thread_local::native::eager::destroy::<core::cell::RefCell<alloc::vec::Vec<alloc::vec::Vec<alloc::vec::Vec<usize>>>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79984772f in __GI___call_tls_dtors () at ./stdlib/cxa_thread_atexit_impl.c:156
#7  0x00007fb79989c96b in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:452
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb78a0f96c0 (LWP 9516) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb7988453e6 in rayon_core::join::join_context::<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb79881efdf in rayon_core::registry::in_worker::<rayon_core::join::join_context<rayon_core::join::join::call<core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, _core::interior_point::kkt::symbolic_with_ordering::{closure#0}>::{closure#0}, rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}, core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64))> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb79897d520 in _core::interior_point::kkt::symbolic_with_ordering () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79893fc2f in <_core::interior_point::kkt::NormalKkt>::new () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb798639909 in _core::interior_point::boxed::solve_box_lp_scaled () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb798638cd1 in _core::interior_point::boxed::solve_box_lp_warm () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb7986664e5 in _core::simplex::crossover::solve_ipm_crossover_with () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb7986625eb in _core::simplex::crossover::solve_ipm_crossover () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb798901a0b in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, _core::simplex::race::solve_race_small::{closure#1}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb798844ad7 in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb798873cd1 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb789df66c0 (LWP 9517) "enomoto-mip-ipm"):
#0  0x00007fb79893341c in faer::sparse::linalg::cholesky::factorize_symbolic_cholesky::<usize> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798865083 in rayon_core::join::join_context::call_b::<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb79888a16e in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), rayon_core::join::join::call<(core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64), _core::interior_point::kkt::symbolic_with_ordering::{closure#1}>::{closure#0}>::{closure#0}, (core::option::Option<faer::sparse::linalg::cholesky::SymbolicCholesky<usize>>, f64)> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d947c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb789bf56c0 (LWP 9518) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d953f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb798d94743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb798d9418b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798d90dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798d971fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7899f46c0 (LWP 9519) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb798dc1ec5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798d97379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb7988faf2c in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79883298c in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb798869edd in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb798dde30f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb79989cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fb799929ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb799ab0740 (LWP 9267) "python3"):
#0  0x00007fb7988153ae in <alloc::collections::btree::map::BTreeMap<usize, usize> as core::ops::drop::Drop>::drop () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb798a74052 in core::ptr::drop_glue::<_core::model::PyModel> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb798ad9d31 in <pyo3::pycell::impl_::PyClassObject<_core::model::PyModel> as pyo3::pycell::impl_::PyClassObjectLayout<_core::model::PyModel>>::tp_dealloc () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb79890bfa1 in pyo3::impl_::trampoline::trampoline_unraisable::<pyo3::impl_::trampoline::dealloc::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb79890c060 in pyo3::impl_::pyclass::tp_dealloc::<_core::model::PyModel> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 9267) detached]
=== stack 8 at 67.376945778s
Could not attach to process.  If your uid matches the uid of the target
process, check the setting of /proc/sys/kernel/yama/ptrace_scope, or try
again as the root user.  For more details, see /etc/sysctl.d/10-ptrace.conf
warning: process 9267 is a zombie - the process has already terminated
ptrace: Inappropriate ioctl for device.
=== stack 9 at 67.447117420s
Could not attach to process.  If your uid matches the uid of the target
process, check the setting of /proc/sys/kernel/yama/ptrace_scope, or try
again as the root user.  For more details, see /etc/sysctl.d/10-ptrace.conf
warning: process 9267 is a zombie - the process has already terminated
ptrace: Inappropriate ioctl for device.
=== stack 10 at 67.535185009s
ptrace: No such process.
