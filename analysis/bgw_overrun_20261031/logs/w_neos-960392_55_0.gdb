=== stack 0 at 56.508322704s
[New LWP 4962]
[New LWP 4948]
[New LWP 4947]
[New LWP 4946]
[New LWP 4945]
[New LWP 4838]
[New LWP 4837]
[New LWP 4836]
[New LWP 4835]
[New LWP 4834]
[New LWP 4833]
[New LWP 4832]
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7f763fbff6c0 (LWP 4832) "python3"):
#0  0x00007f764d298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f7649bbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f7649bbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f7649bbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f764d29b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f7649bbd610 <thread_status+16>, cond=0x7f7649bbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f7649bbd638 <thread_status+56>, mutex=0x7f7649bbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f7648a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7f763f3fe6c0 (LWP 4833) "python3"):
#0  0x00007f764d298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f7649bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f7649bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f7649bbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f764d29b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f7649bbd690 <thread_status+144>, cond=0x7f7649bbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f7649bbd6b8 <thread_status+184>, mutex=0x7f7649bbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f7648a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7f763ebfd6c0 (LWP 4834) "python3"):
#0  0x00007f764d298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f7649bbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f7649bbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f7649bbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f764d29b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f7649bbd710 <thread_status+272>, cond=0x7f7649bbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f7649bbd738 <thread_status+312>, mutex=0x7f7649bbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f7648a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7f763e2fc6c0 (LWP 4835) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7f763e0fb6c0 (LWP 4836) "enomoto-1"):
#0  0x00007f764d298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=4962, futex_word=0x7f75e3fff990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=4962, futex_word=0x7f75e3fff990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f75e3fff990, expected=4962, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f764d29e883 in __pthread_clockjoin_ex (threadid=140144313104064, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f764c14f461 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764bd94e45 in <std::thread::lifecycle::JoinInner<_core::mip::solver::MipResult>>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764b8bf88a in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_workers () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764b91dfd6 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764b88fbad in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f764be440ac in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f764be22fde in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f764be0b176 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f764bc60887 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f764c11fe04 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#19 0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f763defa6c0 (LWP 4837) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f763dbf76c0 (LWP 4838) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f763d3f36c0 (LWP 4945) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f763d1f26c0 (LWP 4946) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f763d5f46c0 (LWP 4947) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f763d9f66c0 (LWP 4948) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c120a33 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764c11fd83 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764c11f7cb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764c11c40a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764c12283d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f75e3fff6c0 (LWP 4962) "enomoto-1"):
#0  0x00007f764be12752 in <std::hash::random::RandomState as core::hash::BuildHasher>::hash_one::<&u32> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f764b9b7f91 in <hashbrown::raw::RawTable<(u32, ())>>::reserve_rehash::<hashbrown::map::make_hasher<u32, (), std::hash::random::RandomState>::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764be1fea8 in <hashbrown::map::HashMap<u32, (), std::hash::random::RandomState>>::insert () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764b9db6e4 in <_core::mip::clique::CliqueGraph>::build_with () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764be2fc4a in _core::mip::extend_clique_rows () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764be4376f in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764be22ee8 in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764b8b4d05 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::solve_submip_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764b8ade58 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::local_branching () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f764b8b63ec in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::alns () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f764b8c349c in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::heur_role_run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f764b914d65 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f764b88fbad in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f764bc0cac6 in _core::cancel::with_halt::<_core::mip::solver::MipResult, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f764bd7929e in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f764bd935ff in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f764c16994f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f764d29cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#18 0x00007f764d329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f764d4e2080 (LWP 4827) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f764c14d505 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f764c1229b9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f764bd60e48 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f764bacf577 in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f764bc412c0 in <_core::model::PyModel>::solve () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f764bc43d6e in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f764bc19fdd in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f764bc5f680 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00000000005cf5b3 in ?? ()
#10 0x000000000056d9e3 in PyObject_Vectorcall ()
#11 0x00000000004400d3 in ?? ()
#12 0x000000000064d96a in PyEval_EvalCode ()
#13 0x000000000068a172 in ?? ()
#14 0x00000000006873de in ?? ()
#15 0x0000000000683d3b in ?? ()
#16 0x0000000000683204 in ?? ()
#17 0x0000000000682ff5 in ?? ()
#18 0x000000000068178c in Py_RunMain ()
#19 0x000000000063cbed in Py_BytesMain ()
#20 0x00007f764d22a1ca in __libc_start_call_main (main=main@entry=0x63cb30, argc=argc@entry=8, argv=argv@entry=0x7fff1313cf58) at ../sysdeps/nptl/libc_start_call_main.h:58
#21 0x00007f764d22a28b in __libc_start_main_impl (main=0x63cb30, argc=8, argv=0x7fff1313cf58, init=<optimized out>, fini=<optimized out>, rtld_fini=<optimized out>, stack_end=0x7fff1313cf48) at ../csu/libc-start.c:360
#22 0x000000000063c085 in _start ()
[Inferior 1 (process 4827) detached]
