=== stack 0 at 61.639129699s
[New LWP 13705]
[New LWP 13704]
[New LWP 13703]
[New LWP 13702]
[New LWP 13044]
[New LWP 13043]
[New LWP 13042]
[New LWP 13041]
[New LWP 12991]
[New LWP 12990]
[New LWP 12989]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7fc9d2dff6c0 (LWP 12989) "python3"):
#0  0x00007fc9dfe98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fc9dcdbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fc9dcdbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fc9dcdbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fc9dfe9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fc9dcdbd610 <thread_status+16>, cond=0x7fc9dcdbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fc9dcdbd638 <thread_status+56>, mutex=0x7fc9dcdbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fc9dbc19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fc9d25fe6c0 (LWP 12990) "python3"):
#0  0x00007fc9dfe98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fc9dcdbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fc9dcdbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fc9dcdbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fc9dfe9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fc9dcdbd690 <thread_status+144>, cond=0x7fc9dcdbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fc9dcdbd6b8 <thread_status+184>, mutex=0x7fc9dcdbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fc9dbc19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fc9d1dfd6c0 (LWP 12991) "python3"):
#0  0x00007fc9dfe98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fc9dcdbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fc9dcdbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fc9dcdbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fc9dfe9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fc9dcdbd710 <thread_status+272>, cond=0x7fc9dcdbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fc9dcdbd738 <thread_status+312>, mutex=0x7fc9dcdbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fc9dbc19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fc9d14fc6c0 (LWP 13041) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fc9d12fb6c0 (LWP 13042) "enomoto-1"):
#0  0x00007fc9defe40b9 in <_core::simplex::lu::FtLu>::u_solve_hyper_heap () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fc9defe8886 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9deff53da in <_core::simplex::lu::FtLu>::solve_sparse_into_pair_capture_tracked () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9dec5e559 in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9deca310b in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9deccf2fc in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9dedff44d in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9dee08662 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9deb6aeeb in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fc9deab64f0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fc9dec39a80 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fc9dec15ba9 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fc9deb30f95 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::maybe_restart () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fc9deb84246 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fc9deab64f0 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fc9dec39a80 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fc9dec15c90 in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fc9dec505c1 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fc9def78f04 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fc9df066b73 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fc9df391544 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#21 0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#22 0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#23 0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#24 0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#25 0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#26 0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fc9d10fa6c0 (LWP 13043) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fc9d0ef96c0 (LWP 13044) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fc9d0cf86c0 (LWP 13702) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fc9d0af76c0 (LWP 13703) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fc9d08f66c0 (LWP 13704) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fc9d02f66c0 (LWP 13705) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df392173 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9df3914c3 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9df390f0b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9df38db4a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9df393f7d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fc9df3db08f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fc9dfe9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fc9dff29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fc9e012c740 (LWP 12983) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fc9df3bec45 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fc9df3940f9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fc9def730a8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fc9def8bf65 in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fc9def8480d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fc9def9f0e0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 12983) detached]
