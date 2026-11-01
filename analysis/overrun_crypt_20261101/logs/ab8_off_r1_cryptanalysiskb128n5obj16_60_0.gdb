=== stack 0 at 61.720003515s
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 8 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda1df8a2b in <_core::mip::domain::Domain>::backtrack_to () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcda1faa1e7 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1fa4a28 in _core::mip::substitute_probe_equiv () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f906dc in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda1f90335 in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1f96cb4 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#17 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 1 at 64.003358642s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda2240212 in <hashbrown::raw::RawTable<((usize, bool), f64)>>::reserve_rehash::<hashbrown::map::make_hasher<(usize, bool), f64, std::hash::random::RandomState>::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcda1f6f5ea in _core::mip::probe::{closure#5} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda1fa8739 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 2 at 66.091853855s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda2240206 in <hashbrown::raw::RawTable<((usize, bool), f64)>>::reserve_rehash::<hashbrown::map::make_hasher<(usize, bool), f64, std::hash::random::RandomState>::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcda1f6f5ea in _core::mip::probe::{closure#5} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda1fa8739 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 3 at 68.179794082s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda34cd931 in ?? ()
#1  0x00007fcda30eca6d in __GI___clock_gettime (clock_id=<optimized out>, tp=<optimized out>) at ../sysdeps/unix/sysv/linux/clock_gettime.c:42
#2  0x00007fcda1fa7583 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 4 at 70.060564916s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda34cd931 in ?? ()
#1  0x00007fcda30eca6d in __GI___clock_gettime (clock_id=<optimized out>, tp=<optimized out>) at ../sysdeps/unix/sysv/linux/clock_gettime.c:42
#2  0x00007fcda1fa7583 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 5 at 72.199783150s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda34cd931 in ?? ()
#1  0x00007fcda30eca6d in __GI___clock_gettime (clock_id=<optimized out>, tp=<optimized out>) at ../sysdeps/unix/sysv/linux/clock_gettime.c:42
#2  0x00007fcda1fa7583 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 6 at 74.400039839s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda1df99db in <_core::mip::domain::Domain>::update_activity () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcda1dfef64 in <_core::mip::domain::Domain>::set_bound () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda1dfe3f4 in <_core::mip::domain::Domain>::propagate () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1fa9e92 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#10 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
=== stack 7 at 76.546768155s
[New LWP 31261]
[New LWP 29947]
[New LWP 29946]
[New LWP 29945]
[New LWP 29944]
[New LWP 29883]
[New LWP 29882]
[New LWP 29881]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7fcd961ff6c0 (LWP 29881) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd610 <thread_status+16>, cond=0x7fcda01bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd638 <thread_status+56>, mutex=0x7fcda01bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7fcd959fe6c0 (LWP 29882) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd690 <thread_status+144>, cond=0x7fcda01bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd6b8 <thread_status+184>, mutex=0x7fcda01bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7fcd951fd6c0 (LWP 29883) "python3"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7fcda01bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcda01bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007fcda309b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7fcda01bd710 <thread_status+272>, cond=0x7fcda01bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7fcda01bd738 <thread_status+312>, mutex=0x7fcda01bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007fcd9f019ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7fcd947fc6c0 (LWP 29944) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7fcd945fb6c0 (LWP 29945) "enomoto-1"):
#0  0x00007fcda3098e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=31261, futex_word=0x7fcd93ff8990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7fcd93ff8990, expected=31261, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007fcda309e883 in __pthread_clockjoin_ex (threadid=140520928020160, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007fcda2579921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda219ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda1d1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda1d4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda1c9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007fcda1f9a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007fcda1f77c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007fcda1fb08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007fcda22236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007fcda22036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007fcda254a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7fcd943fa6c0 (LWP 29946) "enomoto-2"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7fcd941f96c0 (LWP 29947) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda254a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda2549c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda25468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda254ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7fcd93ff86c0 (LWP 31261) "enomoto-1"):
#0  0x00007fcda1f732f5 in <hashbrown::map::HashMap<(usize, bool), f64, std::hash::random::RandomState>>::insert () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007fcda1fa9f42 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda1f907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda1f76c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2197646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda2593e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007fcda309cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007fcda3129ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7fcda33b8740 (LWP 29870) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007fcda25779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007fcda254ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007fcda217ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007fcda21a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007fcda2192d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007fcda21af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 29870) detached]
