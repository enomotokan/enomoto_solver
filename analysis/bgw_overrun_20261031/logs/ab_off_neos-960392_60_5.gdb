=== stack 0 at 61.652280164s
[New LWP 14037]
[New LWP 13626]
[New LWP 13625]
[New LWP 13624]
[New LWP 13623]
[New LWP 13057]
[New LWP 13056]
[New LWP 13055]
[New LWP 13054]
[New LWP 13043]
[New LWP 13042]
[New LWP 13041]
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7fb82bbff6c0 (LWP 13041) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd610 <thread_status+16>, cond=0x7fb835bbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd638 <thread_status+56>, mutex=0x7fb835bbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7fb82b3fe6c0 (LWP 13042) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd690 <thread_status+144>, cond=0x7fb835bbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd6b8 <thread_status+184>, mutex=0x7fb835bbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb82abfd6c0 (LWP 13043) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd710 <thread_status+272>, cond=0x7fb835bbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd738 <thread_status+312>, mutex=0x7fb835bbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb82a3fc6c0 (LWP 13054) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb82a1fb6c0 (LWP 13055) "enomoto-1"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7dbfff990, expected=14037, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fb83929e883 in __pthread_clockjoin_ex (threadid=140427646727872, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fb83814da61 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb8379fa3b5 in <std::thread::lifecycle::JoinInner<_core::mip::solver::MipResult>>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb8378bfdda in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_workers () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837922cac in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378bf5c9 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish_limit () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb83791dbf6 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb837dd6960 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb837db566e in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb837d9b926 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb837d7c313 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb83811e404 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb829ffa6c0 (LWP 13056) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb829cf86c0 (LWP 13057) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb8296f56c0 (LWP 13623) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb8292f36c0 (LWP 13624) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb8294f46c0 (LWP 13625) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb829af76c0 (LWP 13626) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7dbfff6c0 (LWP 14037) "enomoto-1"):
#0  0x00007fb837da8702 in <_core::mip::domain::Domain>::update_activity () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb837dae444 in <_core::mip::domain::Domain>::set_bound () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb837dade82 in <_core::mip::domain::Domain>::propagate () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb837de48ba in _core::mip::probe () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837dcde13 in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837dd404f in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837db5578 in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb8378b5255 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::solve_submip_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378ae3a8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::local_branching () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb8378b693c in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::alns () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378c39ec in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::heur_role_run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb837914df5 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb837d285b6 in _core::cancel::with_halt::<_core::mip::solver::MipResult, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb8379d297e in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb8379f8b6f in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#18 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb839569080 (LWP 13035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb838120fb9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb8379bb1c8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837a6d877 in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837d5d00b in <_core::model::PyModel>::solve () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837d5f7ce in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837d365cd in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb837d7b0e0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
#20 0x00007fb83922a1ca in __libc_start_call_main (main=main@entry=0x63cb30, argc=argc@entry=8, argv=argv@entry=0x7fff6c650058) at ../sysdeps/nptl/libc_start_call_main.h:58
#21 0x00007fb83922a28b in __libc_start_main_impl (main=0x63cb30, argc=8, argv=0x7fff6c650058, init=<optimized out>, fini=<optimized out>, rtld_fini=<optimized out>, stack_end=0x7fff6c650048) at ../csu/libc-start.c:360
#22 0x000000000063c085 in _start ()
[Inferior 1 (process 13035) detached]
=== stack 1 at 63.059774682s
[New LWP 14037]
[New LWP 13626]
[New LWP 13625]
[New LWP 13624]
[New LWP 13623]
[New LWP 13057]
[New LWP 13056]
[New LWP 13055]
[New LWP 13054]
[New LWP 13043]
[New LWP 13042]
[New LWP 13041]
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7fb82bbff6c0 (LWP 13041) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd610 <thread_status+16>, cond=0x7fb835bbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd638 <thread_status+56>, mutex=0x7fb835bbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7fb82b3fe6c0 (LWP 13042) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd690 <thread_status+144>, cond=0x7fb835bbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd6b8 <thread_status+184>, mutex=0x7fb835bbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb82abfd6c0 (LWP 13043) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd710 <thread_status+272>, cond=0x7fb835bbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd738 <thread_status+312>, mutex=0x7fb835bbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb82a3fc6c0 (LWP 13054) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb82a1fb6c0 (LWP 13055) "enomoto-1"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7dbfff990, expected=14037, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fb83929e883 in __pthread_clockjoin_ex (threadid=140427646727872, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fb83814da61 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb8379fa3b5 in <std::thread::lifecycle::JoinInner<_core::mip::solver::MipResult>>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb8378bfdda in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_workers () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837922cac in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378bf5c9 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish_limit () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb83791dbf6 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb837dd6960 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb837db566e in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb837d9b926 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb837d7c313 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb83811e404 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb829ffa6c0 (LWP 13056) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb829cf86c0 (LWP 13057) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb8296f56c0 (LWP 13623) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb8292f36c0 (LWP 13624) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb8294f46c0 (LWP 13625) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb829af76c0 (LWP 13626) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7dbfff6c0 (LWP 14037) "enomoto-1"):
#0  0x00007fb837e77f60 in mi_malloc_aligned () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb837c1de9a in <hashbrown::raw::RawTableInner>::fallible_with_capacity::<alloc::alloc::Global> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb837c197f3 in <hashbrown::raw::RawTable<(u32, ())>>::reserve_rehash::<hashbrown::map::make_hasher<u32, (), std::hash::random::RandomState>::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb837db0ad8 in <hashbrown::map::HashMap<u32, (), std::hash::random::RandomState>>::insert () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837c2abd1 in <_core::mip::clique::CliqueGraph>::build_with_stop () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837dc22f5 in _core::mip::extend_clique_rows () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837dd5ffd in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837db5578 in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378b5255 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::solve_submip_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb8378ae3a8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::local_branching () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378b693c in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::alns () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb8378c39ec in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::heur_role_run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb837914df5 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb837d285b6 in _core::cancel::with_halt::<_core::mip::solver::MipResult, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb8379d297e in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb8379f8b6f in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#19 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb839569080 (LWP 13035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb838120fb9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb8379bb1c8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837a6d877 in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837d5d00b in <_core::model::PyModel>::solve () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837d5f7ce in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837d365cd in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb837d7b0e0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
#20 0x00007fb83922a1ca in __libc_start_call_main (main=main@entry=0x63cb30, argc=argc@entry=8, argv=argv@entry=0x7fff6c650058) at ../sysdeps/nptl/libc_start_call_main.h:58
#21 0x00007fb83922a28b in __libc_start_main_impl (main=0x63cb30, argc=8, argv=0x7fff6c650058, init=<optimized out>, fini=<optimized out>, rtld_fini=<optimized out>, stack_end=0x7fff6c650048) at ../csu/libc-start.c:360
#22 0x000000000063c085 in _start ()
[Inferior 1 (process 13035) detached]
=== stack 2 at 64.439534605s
[New LWP 14037]
[New LWP 13626]
[New LWP 13625]
[New LWP 13624]
[New LWP 13623]
[New LWP 13057]
[New LWP 13056]
[New LWP 13055]
[New LWP 13054]
[New LWP 13043]
[New LWP 13042]
[New LWP 13041]
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 13 (Thread 0x7fb82bbff6c0 (LWP 13041) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd610 <thread_status+16>, cond=0x7fb835bbd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd638 <thread_status+56>, mutex=0x7fb835bbd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 12 (Thread 0x7fb82b3fe6c0 (LWP 13042) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd690 <thread_status+144>, cond=0x7fb835bbd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd6b8 <thread_status+184>, mutex=0x7fb835bbd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 11 (Thread 0x7fb82abfd6c0 (LWP 13043) "python3"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fb835bbd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb835bbd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fb83929b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fb835bbd710 <thread_status+272>, cond=0x7fb835bbd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fb835bbd738 <thread_status+312>, mutex=0x7fb835bbd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fb834a19ad3 in blas_thread_server () from /usr/local/lib/python3.13/dist-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 10 (Thread 0x7fb82a3fc6c0 (LWP 13054) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 9 (Thread 0x7fb82a1fb6c0 (LWP 13055) "enomoto-1"):
#0  0x00007fb839298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=14037, futex_word=0x7fb7dbfff990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fb7dbfff990, expected=14037, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fb83929e883 in __pthread_clockjoin_ex (threadid=140427646727872, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fb83814da61 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb8379fa3b5 in <std::thread::lifecycle::JoinInner<_core::mip::solver::MipResult>>::join () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb8378bfdda in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_workers () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837922cac in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378bf5c9 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::finish_limit () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb83791dbf6 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb837dd6960 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb837db566e in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb837d9b926 in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb837d7c313 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb83811e404 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#20 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#21 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fb829ffa6c0 (LWP 13056) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fb829cf86c0 (LWP 13057) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fb8296f56c0 (LWP 13623) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fb8292f36c0 (LWP 13624) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fb8294f46c0 (LWP 13625) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fb829af76c0 (LWP 13626) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb83811f033 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb83811e383 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb83811ddcb in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb83811aa0a in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb838120e3d in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fb7dbfff6c0 (LWP 14037) "enomoto-1"):
#0  0x00007fb8379908a6 in <alloc::vec::Vec<u32> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<u32, std::collections::hash::set::IntoIter<u32>>>::from_iter () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fb8379f376f in <alloc::vec::into_iter::IntoIter<std::collections::hash::set::HashSet<u32>> as core::iter::traits::iterator::Iterator>::try_fold::<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<u32>>, core::iter::adapters::map::map_try_fold<std::collections::hash::set::HashSet<u32>, alloc::vec::Vec<u32>, alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<u32>>, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<u32>>, !>, <_core::mip::clique::CliqueGraph>::build_with_stop::{closure#1}, alloc::vec::in_place_collect::write_in_place_with_drop<alloc::vec::Vec<u32>>::{closure#0}>::{closure#0}, core::result::Result<alloc::vec::in_place_drop::InPlaceDrop<alloc::vec::Vec<u32>>, !>> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb8379cbaea in alloc::vec::in_place_collect::from_iter_in_place::<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<std::collections::hash::set::HashSet<u32>>, <_core::mip::clique::CliqueGraph>::build_with_stop::{closure#1}>, alloc::vec::Vec<u32>> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb837c2accc in <_core::mip::clique::CliqueGraph>::build_with_stop () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837dc22f5 in _core::mip::extend_clique_rows () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837dd5ffd in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837db5578 in _core::mip::solve_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb8378b5255 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::solve_submip_problem () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb8378ae3a8 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::local_branching () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fb8378b693c in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::alns () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fb8378c39ec in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::heur_role_run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fb837914df5 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fb8378900fd in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fb837d285b6 in _core::cancel::with_halt::<_core::mip::solver::MipResult, <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fb8379d297e in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fb8379f8b6f in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::start_workers::{closure#3}, _core::mip::solver::MipResult>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fb838167f4f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fb83929cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#18 0x00007fb839329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fb839569080 (LWP 13035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fb83814bb05 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fb838120fb9 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fb8379bb1c8 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fb837a6d877 in <pyo3::marker::Python>::allow_threads::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fb837d5d00b in <_core::model::PyModel>::solve () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fb837d5f7ce in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fb837d365cd in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fb837d7b0e0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/dist-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
#20 0x00007fb83922a1ca in __libc_start_call_main (main=main@entry=0x63cb30, argc=argc@entry=8, argv=argv@entry=0x7fff6c650058) at ../sysdeps/nptl/libc_start_call_main.h:58
#21 0x00007fb83922a28b in __libc_start_main_impl (main=0x63cb30, argc=8, argv=0x7fff6c650058, init=<optimized out>, fini=<optimized out>, rtld_fini=<optimized out>, stack_end=0x7fff6c650048) at ../csu/libc-start.c:360
#22 0x000000000063c085 in _start ()
[Inferior 1 (process 13035) detached]
