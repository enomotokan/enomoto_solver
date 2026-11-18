=== stack 0 at 205.043805593s
[New LWP 25506]
[New LWP 25501]
[New LWP 25500]
[New LWP 25499]
[New LWP 25498]
[New LWP 21591]
[New LWP 21590]
[New LWP 21589]
[New LWP 21588]
[New LWP 21317]
[New LWP 21316]
[New LWP 21315]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7f72253ff6c0 (LWP 21315) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd610 <thread_status+16>, cond=0x7f722f3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd638 <thread_status+56>, mutex=0x7f722f3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7f7224bfe6c0 (LWP 21316) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd690 <thread_status+144>, cond=0x7f722f3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd6b8 <thread_status+184>, mutex=0x7f722f3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7f72243fd6c0 (LWP 21317) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd710 <thread_status+272>, cond=0x7f722f3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd738 <thread_status+312>, mutex=0x7f722f3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7f7223bfc6c0 (LWP 21588) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316c7260 in <rayon_core::registry::Registry>::in_worker_cross::<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317b3a37 in <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f72317b9a91 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723116e527 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231013ff5 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f723168a7a6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f723164920c in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f72316364ae in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f72316aa855 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f72316e287e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f7231745633 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7f72239fb6c0 (LWP 21589) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f72237fa6c0 (LWP 21590) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f72235f96c0 (LWP 21591) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f72225f36c0 (LWP 25498) "enomoto-mip-sim"):
#0  0x00007f723192f18b in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f723198388e in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72317aa6d2 in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72317a8296 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183c5c3 in _core::cancel::with_token::<_core::mip::lp::LpStatus, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170e7ff in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231751503 in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, <rayon_core::registry::Registry>::in_worker_cross<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f72223f26c0 (LWP 25499) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f72221f16c0 (LWP 25500) "enomoto-mip-ipm"):
#0  0x00007f723185c56d in <_core::simplex::crossover::BasisSelector>::try_add () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f72318e0a93 in _core::simplex::crossover::solve_ipm_crossover_with () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72318cedeb in _core::simplex::crossover::solve_ipm_crossover () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231820f17 in _core::simplex::mip_lp::ipm_crossover_run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183cdd4 in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170d91d in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231744c3b in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f72227f46c0 (LWP 25501) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f7222df76c0 (LWP 25506) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ace22 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316ef9fd in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317354ab in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f7232a79740 (LWP 21300) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ad4f8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72317cdc19 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317d533e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f723177143d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231821820 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 21300) detached]
=== stack 1 at 205.458302774s
[New LWP 25506]
[New LWP 25501]
[New LWP 25500]
[New LWP 25499]
[New LWP 25498]
[New LWP 21591]
[New LWP 21590]
[New LWP 21589]
[New LWP 21588]
[New LWP 21317]
[New LWP 21316]
[New LWP 21315]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7f72253ff6c0 (LWP 21315) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd610 <thread_status+16>, cond=0x7f722f3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd638 <thread_status+56>, mutex=0x7f722f3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7f7224bfe6c0 (LWP 21316) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd690 <thread_status+144>, cond=0x7f722f3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd6b8 <thread_status+184>, mutex=0x7f722f3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7f72243fd6c0 (LWP 21317) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd710 <thread_status+272>, cond=0x7f722f3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd738 <thread_status+312>, mutex=0x7f722f3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7f7223bfc6c0 (LWP 21588) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316c7260 in <rayon_core::registry::Registry>::in_worker_cross::<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317b3a37 in <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f72317b9a91 in <_core::simplex::mip_lp::TwoStageLp>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723116e527 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231013ff5 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f723168a7a6 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f723164920c in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f72316364ae in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f72316aa855 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f72316e287e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f7231745633 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7f72239fb6c0 (LWP 21589) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f72237fa6c0 (LWP 21590) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f72235f96c0 (LWP 21591) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f72225f36c0 (LWP 25498) "enomoto-mip-sim"):
#0  0x00007f72315f74d9 in <_core::simplex::lu::FtLu>::permute_out () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f7231604e57 in <_core::simplex::lu::FtLu>::pair_r_u_permute_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f723160add5 in <_core::simplex::lu::FtLu>::solve_into_pair_capture_tracked () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72318f0a5e in <_core::simplex::basis_kernel::BasisKernel>::ftran_fused::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723193e709 in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual_impl::<true> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723198388e in _core::simplex::slope_intercept_dual::solve_slope_intercept_dual () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f72317aa6d2 in <_core::simplex::mip_lp::TwoStageLp>::solve_inner0 () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f72317a8296 in <_core::simplex::mip_lp::TwoStageLp>::solve_once () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723183c5c3 in _core::cancel::with_token::<_core::mip::lp::LpStatus, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f723170e7ff in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231751503 in <rayon_core::job::StackJob<rayon_core::latch::SpinLatch, <rayon_core::registry::Registry>::in_worker_cross<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#3}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])>::{closure#0}, (_core::mip::lp::LpStatus, u8, core::option::Option<alloc::vec::Vec<f64>>, [u64; 9])> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#17 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f72223f26c0 (LWP 25499) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f72221f16c0 (LWP 25500) "enomoto-mip-ipm"):
#0  0x00007f723185c566 in <_core::simplex::crossover::BasisSelector>::try_add () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f72318e0a93 in _core::simplex::crossover::solve_ipm_crossover_with () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72318cedeb in _core::simplex::crossover::solve_ipm_crossover () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231820f17 in _core::simplex::mip_lp::ipm_crossover_run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183cdd4 in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170d91d in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231744c3b in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f72227f46c0 (LWP 25501) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f7222df76c0 (LWP 25506) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ace22 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316ef9fd in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317354ab in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f7232a79740 (LWP 21300) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ad4f8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72317cdc19 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317d533e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f723177143d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231821820 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 21300) detached]
=== stack 2 at 205.841550299s
[New LWP 25506]
[New LWP 25501]
[New LWP 25500]
[New LWP 25499]
[New LWP 21591]
[New LWP 21590]
[New LWP 21589]
[New LWP 21588]
[New LWP 21317]
[New LWP 21316]
[New LWP 21315]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 12 (Thread 0x7f72253ff6c0 (LWP 21315) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd610 <thread_status+16>, cond=0x7f722f3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd638 <thread_status+56>, mutex=0x7f722f3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7f7224bfe6c0 (LWP 21316) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd690 <thread_status+144>, cond=0x7f722f3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd6b8 <thread_status+184>, mutex=0x7f722f3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7f72243fd6c0 (LWP 21317) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd710 <thread_status+272>, cond=0x7f722f3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd738 <thread_status+312>, mutex=0x7f722f3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7f7223bfc6c0 (LWP 21588) "enomoto-0"):
#0  0x00007f72319a1d03 in mi_free () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f723162cc63 in core::ptr::drop_glue::<_core::mip::problem::MipProblem> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f723168ab31 in _core::mip::solve_problem_inner () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f723164920c in _core::mip::solve_problem () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316364ae in _core::mip::solve_mip::{closure#12} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72316aa855 in _core::mip::solve_mip () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f72316e287e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231745633 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#14 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f72239fb6c0 (LWP 21589) "enomoto-1"):
#0  0x00007f723290e8cb in __GI_sched_yield () at ../sysdeps/unix/syscall-template.S:120
#1  0x00007f7231c4a727 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f72237fa6c0 (LWP 21590) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f72235f96c0 (LWP 21591) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f72223f26c0 (LWP 25499) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f72221f16c0 (LWP 25500) "enomoto-mip-ipm"):
#0  0x00007f723185c8a7 in <_core::simplex::crossover::BasisSelector>::try_add () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f72318e0a93 in _core::simplex::crossover::solve_ipm_crossover_with () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72318cedeb in _core::simplex::crossover::solve_ipm_crossover () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231820f17 in _core::simplex::mip_lp::ipm_crossover_run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183cdd4 in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170d91d in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231744c3b in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f72227f46c0 (LWP 25501) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f7222df76c0 (LWP 25506) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ace22 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316ef9fd in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317354ab in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f7232a79740 (LWP 21300) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ad4f8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72317cdc19 in <_core::model::PyModel>::solve () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317d533e in <_core::model::PyModel>::__pymethod_solve__ () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f723177143d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231821820 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 21300) detached]
=== stack 3 at 206.267034277s
[New LWP 25506]
[New LWP 25501]
[New LWP 25500]
[New LWP 25499]
[New LWP 21317]
[New LWP 21316]
[New LWP 21315]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x00007f72328a9bbd in unlink_chunk (p=p@entry=0x39025c30, av=<optimized out>) at ./malloc/malloc.c:1616

warning: 1616	./malloc/malloc.c: No such file or directory

Thread 8 (Thread 0x7f72253ff6c0 (LWP 21315) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd610 <thread_status+16>, cond=0x7f722f3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd638 <thread_status+56>, mutex=0x7f722f3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f7224bfe6c0 (LWP 21316) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd690 <thread_status+144>, cond=0x7f722f3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd6b8 <thread_status+184>, mutex=0x7f722f3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f72243fd6c0 (LWP 21317) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd710 <thread_status+272>, cond=0x7f722f3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd738 <thread_status+312>, mutex=0x7f722f3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f72223f26c0 (LWP 25499) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f72221f16c0 (LWP 25500) "enomoto-mip-ipm"):
#0  0x00007f723185c566 in <_core::simplex::crossover::BasisSelector>::try_add () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f72318e0a93 in _core::simplex::crossover::solve_ipm_crossover_with () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72318cedeb in _core::simplex::crossover::solve_ipm_crossover () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231820f17 in _core::simplex::mip_lp::ipm_crossover_run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183cdd4 in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170d91d in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231744c3b in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f72227f46c0 (LWP 25501) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f7222df76c0 (LWP 25506) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ace22 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316ef9fd in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317354ab in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f7232a79740 (LWP 21300) "python3"):
#0  0x00007f72328a9bbd in unlink_chunk (p=p@entry=0x39025c30, av=<optimized out>) at ./malloc/malloc.c:1616
#1  0x00007f72328a9d9a in malloc_consolidate (av=0x7f7232a04ac0 <main_arena>) at ./malloc/malloc.c:4868
#2  0x00007f72328aae65 in _int_free_maybe_consolidate (av=0x7f7232a04ac0 <main_arena>, size=<optimized out>) at ./malloc/malloc.c:4779
#3  0x00007f72328ab4fa in _int_free (av=0x7f7232a04ac0 <main_arena>, p=<optimized out>, have_lock=<optimized out>) at ./malloc/malloc.c:4646
#4  0x00007f72328ade8e in __GI___libc_free (mem=0x3efd04f0) at ./malloc/malloc.c:3398
#5  0x00007f7231dfb18f in ?? () from /usr/local/lib/python3.13/site-packages/highspy/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231e2908f in ?? () from /usr/local/lib/python3.13/site-packages/highspy/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231de4d11 in ?? () from /usr/local/lib/python3.13/site-packages/highspy/_core.cpython-313-x86_64-linux-gnu.so
#8  0x0000000001611a6f in _PyEval_EvalFrameDefault ()
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
[Inferior 1 (process 21300) detached]
=== stack 4 at 206.686742081s
[New LWP 25506]
[New LWP 25501]
[New LWP 25500]
[New LWP 25499]
[New LWP 21317]
[New LWP 21316]
[New LWP 21315]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
0x0000000001644b56 in gc_collect_main ()

Thread 8 (Thread 0x7f72253ff6c0 (LWP 21315) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd610 <thread_status+16>, cond=0x7f722f3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd638 <thread_status+56>, mutex=0x7f722f3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f7224bfe6c0 (LWP 21316) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd690 <thread_status+144>, cond=0x7f722f3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd6b8 <thread_status+184>, mutex=0x7f722f3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f72243fd6c0 (LWP 21317) "python3"):
#0  0x00007f7232898e31 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f722f3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f722f3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f723289b8ad in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f722f3bd710 <thread_status+272>, cond=0x7f722f3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f722f3bd738 <thread_status+312>, mutex=0x7f722f3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f722e219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f72223f26c0 (LWP 25499) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f72221f16c0 (LWP 25500) "enomoto-mip-ipm"):
#0  0x00007f723185c881 in <_core::simplex::crossover::BasisSelector>::try_add () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f72318e0a93 in _core::simplex::crossover::solve_ipm_crossover_with () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f72318cedeb in _core::simplex::crossover::solve_ipm_crossover () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231820f17 in _core::simplex::mip_lp::ipm_crossover_run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f723183cdd4 in _core::cancel::with_deadline::<core::option::Option<_core::simplex::SimplexResult>, <_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}::{closure#1}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f723170d91d in <rayon_core::thread_pool::ThreadPool>::install::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231744c3b in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> as rayon_core::job::Job>::execute () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c4a7c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#13 0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f72227f46c0 (LWP 25501) "enomoto-mip-ipm"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4b3f3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f7231c4a743 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f7231c4a18b in <rayon_core::registry::ThreadBuilder>::run () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f7231c46dca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c4d1fd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f7222df76c0 (LWP 25506) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f7231c78025 in <std::sys::sync::condvar::futex::Condvar>::wait () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f7231c4d379 in <rayon_core::latch::LockLatch>::wait_and_reset () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f72314ace22 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>>::{closure#0}, core::option::Option<(alloc::vec::Vec<f64>, core::option::Option<(alloc::vec::Vec<f64>, alloc::vec::Vec<core::option::Option<usize>>)>)>> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f72316ef9fd in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()> () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f72317354ab in <std::thread::lifecycle::spawn_unchecked<<_core::simplex::mip_lp::TwoStageLp>::solve_race_ext::{closure#2}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f7231c944af in <std::sys::thread::unix::Thread>::new::thread_start () from /home/user/enomoto_solver/python/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f723289cb64 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f7232929e8c in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f7232a79740 (LWP 21300) "python3"):
#0  0x0000000001644b56 in gc_collect_main ()
#1  0x000000000179063f in finalize_modules ()
#2  0x000000000178fa89 in _Py_Finalize ()
#3  0x0000000001787706 in Py_RunMain ()
#4  0x0000000001787454 in pymain_main ()
#5  0x00000000017872b8 in main ()
[Inferior 1 (process 21300) detached]
=== stack 5 at 207.047677229s
Could not attach to process.  If your uid matches the uid of the target
process, check the setting of /proc/sys/kernel/yama/ptrace_scope, or try
again as the root user.  For more details, see /etc/sysctl.d/10-ptrace.conf
warning: process 21300 is a zombie - the process has already terminated
ptrace: Inappropriate ioctl for device.
=== stack 6 at 207.094340964s
ptrace: No such process.
