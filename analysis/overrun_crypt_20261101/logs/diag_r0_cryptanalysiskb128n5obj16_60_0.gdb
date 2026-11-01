=== stack 0 at 61.682447797s
[New LWP 11510]
[New LWP 11508]
[New LWP 11507]
[New LWP 10367]
[New LWP 10366]
[New LWP 10365]
[New LWP 10364]
[New LWP 10333]
[New LWP 10332]
[New LWP 10331]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 11 (Thread 0x7f1eaadff6c0 (LWP 10331) "python3"):
#0  0x00007f1eb7c98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1eb4dbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1eb4dbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1eb4dbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1eb7c9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1eb4dbd610 <thread_status+16>, cond=0x7f1eb4dbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1eb4dbd638 <thread_status+56>, mutex=0x7f1eb4dbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1eb3c19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7f1eaa5fe6c0 (LWP 10332) "python3"):
#0  0x00007f1eb7c98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1eb4dbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1eb4dbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1eb4dbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1eb7c9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1eb4dbd690 <thread_status+144>, cond=0x7f1eb4dbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1eb4dbd6b8 <thread_status+184>, mutex=0x7f1eb4dbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1eb3c19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7f1ea9dfd6c0 (LWP 10333) "python3"):
#0  0x00007f1eb7c98e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f1eb4dbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f1eb4dbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1eb4dbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f1eb7c9b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f1eb4dbd710 <thread_status+272>, cond=0x7f1eb4dbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f1eb4dbd738 <thread_status+312>, mutex=0x7f1eb4dbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f1eb3c19ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f1ea94fc6c0 (LWP 10364) "enomoto-0"):
#0  0x00007f1eb7d0e90b in __GI_sched_yield () at ../sysdeps/unix/syscall-template.S:120
#1  0x00007f1eb7349ce7 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1eb734974b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1eb734638a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1eb734c7bd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb73938cf in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f1ea92fb6c0 (LWP 10365) "enomoto-1"):
#0  0x00007f1eb7c98e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=11507, futex_word=0x7f1ea8cf8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=11507, futex_word=0x7f1ea8cf8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f1ea8cf8990, expected=11507, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f1eb7c9e883 in __pthread_clockjoin_ex (threadid=139769657919168, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f1eb73793e1 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb6ed6cb5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb6b1e706 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f1eb6b474ed in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f1eb6a9d4c9 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f1eb6cbb80c in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f1eb6c9957e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f1eb6cd1ea4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f1eb6eef22e in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f1eb7019853 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f1eb7349d84 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f1eb734974b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f1eb734638a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f1eb734c7bd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f1eb73938cf in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f1ea90fa6c0 (LWP 10366) "enomoto-2"):
#0  0x00007f1eb7d0e90b in __GI_sched_yield () at ../sysdeps/unix/syscall-template.S:120
#1  0x00007f1eb7349ce7 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1eb734974b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1eb734638a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1eb734c7bd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb73938cf in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f1ea8ef96c0 (LWP 10367) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f1eb7377485 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1eb734a9b3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1eb7349d03 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1eb734974b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb734638a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb734c7bd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f1eb73938cf in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f1ea8cf86c0 (LWP 11507) "enomoto-1"):
#0  __memcpy_avx512_unaligned_erms () at ../sysdeps/x86_64/multiarch/memmove-vec-unaligned-erms.S:266
#1  0x00007f1eb70a17ce in _mi_theap_realloc_zero () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1eb7010eee in <alloc::raw_vec::RawVecInner>::finish_grow () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1eb700fd3a in <alloc::raw_vec::RawVec<u32>>::grow_one () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1eb6bf9c4e in <_core::mip::domain::Domain>::add_implications () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb6ccb297 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb6cb202a in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f1eb6cc61c8 in _core::mip::substitute_probe_equiv () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f1eb6cb1f4c in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f1eb6c98569 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f1eb6fe577d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f1eb6ed3516 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f1eb73938cf in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f1eb7c9cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#14 0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f1ea83f76c0 (LWP 11508) "enomoto-mip-sim"):
#0  0x00007f1eb7d2538b in __GI_madvise () at ../sysdeps/unix/syscall-template.S:117
#1  0x00007f1eb7caae5c in shrink_heap (diff=66879488, h=0x7f1e78000000) at ./malloc/arena.c:525
#2  heap_trim (pad=<optimized out>, heap=0x7f1e78000000) at ./malloc/arena.c:600
#3  _int_free_maybe_consolidate (av=0x7f1e78000030, size=<optimized out>) at ./malloc/malloc.c:4795
#4  0x00007f1eb7cab51a in _int_free (av=0x7f1e78000030, p=<optimized out>, have_lock=<optimized out>) at ./malloc/malloc.c:4646
#5  0x00007f1eb7cae101 in __GI___libc_free (mem=<optimized out>) at ./malloc/malloc.c:3398
#6  tcache_thread_shutdown () at ./malloc/malloc.c:3234
#7  __malloc_arena_thread_freeres () at ./malloc/arena.c:897
#8  0x00007f1eb7cb0602 in __libc_thread_freeres () at ./malloc/thread-freeres.c:41
#9  0x00007f1eb7c9c975 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:458
#10 0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f1ea8af76c0 (LWP 11510) "enomoto-mip-ipm"):
#0  0x00007f1eb7d2538b in __GI_madvise () at ../sysdeps/unix/syscall-template.S:117
#1  0x00007f1eb7caae5c in shrink_heap (diff=66940928, h=0x7f1e7c000000) at ./malloc/arena.c:525
#2  heap_trim (pad=<optimized out>, heap=0x7f1e7c000000) at ./malloc/arena.c:600
#3  _int_free_maybe_consolidate (av=0x7f1e7c000030, size=<optimized out>) at ./malloc/malloc.c:4795
#4  0x00007f1eb7cab51a in _int_free (av=0x7f1e7c000030, p=<optimized out>, have_lock=<optimized out>) at ./malloc/malloc.c:4646
#5  0x00007f1eb7cae101 in __GI___libc_free (mem=<optimized out>) at ./malloc/malloc.c:3398
#6  tcache_thread_shutdown () at ./malloc/malloc.c:3234
#7  __malloc_arena_thread_freeres () at ./malloc/arena.c:897
#8  0x00007f1eb7cb0602 in __libc_thread_freeres () at ./malloc/thread-freeres.c:41
#9  0x00007f1eb7c9c975 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:458
#10 0x00007f1eb7d29ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f1eb7fe5740 (LWP 10320) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f1eb7377485 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f1eb734c939 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f1eb6eb6938 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f1eb6edd720 in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f1eb6ecea6d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f1eb6eeb180 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 10320) detached]
