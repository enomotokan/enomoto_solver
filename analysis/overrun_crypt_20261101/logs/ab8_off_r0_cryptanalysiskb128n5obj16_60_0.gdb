=== stack 0 at 61.521083955s
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 8 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f090fff9a67 in <_core::mip::domain::Domain>::update_activity () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f090fffef64 in <_core::mip::domain::Domain>::set_bound () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f090fffe3f4 in <_core::mip::domain::Domain>::propagate () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f09101aa0e6 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09101907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09101a4a28 in _core::mip::substitute_probe_equiv () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09101906dc in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910190335 in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f0910196cb4 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#19 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
=== stack 1 at 63.560447339s
[New LWP 25421]
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f0901df6990, expected=25421, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f091129e883 in __pthread_clockjoin_ex (threadid=139676662851264, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f0910779921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f091039ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f090ff1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f090ff4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f090fe9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f091019a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f0901df66c0 (LWP 25421) "enomoto-2"):
#0  0x00007f09101c3357 in <core::hash::sip::Hasher<core::hash::sip::Sip13Rounds> as core::hash::Hasher>::write () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f091016d30c in <std::hash::random::RandomState as core::hash::BuildHasher>::hash_one::<&alloc::vec::Vec<(usize, u64)>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091043cfe4 in <hashbrown::raw::RawTable<(alloc::vec::Vec<(usize, i64)>, (usize, f64, f64, f64))>>::reserve_rehash::<hashbrown::map::make_hasher<alloc::vec::Vec<(usize, i64)>, (usize, f64, f64, f64), std::hash::random::RandomState>::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091018d908 in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910176c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09103cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f0910397646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
=== stack 2 at 65.601325613s
[New LWP 25421]
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f0901df6990, expected=25421, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f091129e883 in __pthread_clockjoin_ex (threadid=139676662851264, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f0910779921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f091039ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f090ff1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f090ff4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f090fe9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f091019a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f0901df66c0 (LWP 25421) "enomoto-2"):
#0  0x00007f090fffe653 in <_core::mip::domain::Domain>::propagate () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#1  0x00007f09101a7ab1 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f09101907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f0910176c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910397646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#8  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
=== stack 3 at 67.892338682s
[New LWP 25421]
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f0901df6990, expected=25421, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f091129e883 in __pthread_clockjoin_ex (threadid=139676662851264, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f0910779921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f091039ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f090ff1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f090ff4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f090fe9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f091019a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f0901df66c0 (LWP 25421) "enomoto-2"):
#0  0x00007f09116c0931 in ?? ()
#1  0x00007f09112eca6d in __GI___clock_gettime (clock_id=<optimized out>, tp=<optimized out>) at ../sysdeps/unix/sysv/linux/clock_gettime.c:42
#2  0x00007f09101a7583 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f09101907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910176c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09103cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f0910397646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
=== stack 4 at 70.128690422s
[New LWP 25421]
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f0901df6990, expected=25421, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f091129e883 in __pthread_clockjoin_ex (threadid=139676662851264, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f0910779921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f091039ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f090ff1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f090ff4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f090fe9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f091019a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f0901df66c0 (LWP 25421) "enomoto-2"):
#0  0x00007f09116c0931 in ?? ()
#1  0x00007f09112eca6d in __GI___clock_gettime (clock_id=<optimized out>, tp=<optimized out>) at ../sysdeps/unix/sysv/linux/clock_gettime.c:42
#2  0x00007f09101a7583 in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f09101907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910176c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09103cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f0910397646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
=== stack 5 at 72.180686367s
[New LWP 25421]
[New LWP 24108]
[New LWP 24107]
[New LWP 24106]
[New LWP 24105]
[New LWP 24043]
[New LWP 24042]
[New LWP 24041]

warning: Loadable section ".dynstr" outside of ELF segments
  in /usr/local/bin/python3.13
[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/x86_64-linux-gnu/libthread_db.so.1".
syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38

warning: 38	../sysdeps/unix/sysv/linux/x86_64/syscall.S: No such file or directory

Thread 9 (Thread 0x7f09043ff6c0 (LWP 24041) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd660 <thread_status+96>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd660 <thread_status+96>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd610 <thread_status+16>, cond=0x7f090e3bd638 <thread_status+56>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd638 <thread_status+56>, mutex=0x7f090e3bd610 <thread_status+16>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 8 (Thread 0x7f0903bfe6c0 (LWP 24042) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd6e0 <thread_status+224>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd6e0 <thread_status+224>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd690 <thread_status+144>, cond=0x7f090e3bd6b8 <thread_status+184>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd6b8 <thread_status+184>, mutex=0x7f090e3bd690 <thread_status+144>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 7 (Thread 0x7f09033fd6c0 (LWP 24043) "python3"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0x0, op=393, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0x0, clockid=0, expected=0, futex_word=0x7f090e3bd760 <thread_status+352>) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f090e3bd760 <thread_status+352>, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x00007f091129b8cd in __pthread_cond_wait_common (abstime=0x0, clockid=0, mutex=0x7f090e3bd710 <thread_status+272>, cond=0x7f090e3bd738 <thread_status+312>) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_wait (cond=0x7f090e3bd738 <thread_status+312>, mutex=0x7f090e3bd710 <thread_status+272>) at ./nptl/pthread_cond_wait.c:627
#5  0x00007f090d219ad3 in blas_thread_server () from /usr/local/lib/python3.13/site-packages/numpy/_core/../../numpy.libs/libscipy_openblas64_-f48b354e.so
#6  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#7  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 6 (Thread 0x7f09029fc6c0 (LWP 24105) "enomoto-0"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 5 (Thread 0x7f09027fb6c0 (LWP 24106) "enomoto-1"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 4 (Thread 0x7f09025fa6c0 (LWP 24107) "enomoto-2"):
#0  0x00007f0911298e51 in __futex_abstimed_wait_common64 (private=128, cancel=true, abstime=0x0, op=265, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=128, abstime=0x0, clockid=0, expected=25421, futex_word=0x7f0901df6990) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0x7f0901df6990, expected=25421, clockid=clockid@entry=0, abstime=abstime@entry=0x0, private=private@entry=128) at ./nptl/futex-internal.c:139
#3  0x00007f091129e883 in __pthread_clockjoin_ex (threadid=139676662851264, thread_return=0x0, clockid=0, abstime=0x0, block=<optimized out>) at ./nptl/pthread_join_common.c:102
#4  0x00007f0910779921 in <std::sys::thread::unix::Thread>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f091039ade5 in <std::thread::lifecycle::JoinInner<core::option::Option<alloc::vec::Vec<f64>>>>::join () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f090ff1e866 in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::join_fj_thread () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f090ff4764d in <_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f090fe9d599 in _core::mip::solver::solve_with::<_core::simplex::mip_lp::TwoStageLp> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f091019a1a8 in _core::mip::solve_problem_inner () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f0910177c1e in _core::mip::solve_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#11 0x00007f09101b08b4 in _core::mip::solve_mip () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#12 0x00007f09104236ae in _core::log::with_sink::<_core::types::SolveResult, <_core::model::PyModel>::solve::{closure#1}::{closure#0}::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#13 0x00007f09104036a3 in <rayon_core::job::StackJob<rayon_core::latch::LatchRef<rayon_core::latch::LockLatch>, <rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}::{closure#0}, _core::types::SolveResult> as rayon_core::job::Job>::execute () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#14 0x00007f091074a2c4 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#15 0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#16 0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#17 0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#18 0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#19 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#20 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 3 (Thread 0x7f09023f96c0 (LWP 24108) "enomoto-3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074aef3 in <rayon_core::sleep::Sleep>::sleep::<<rayon_core::registry::WorkerThread>::wait_until_cold::{closure#0}> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091074a243 in <rayon_core::registry::WorkerThread>::wait_until_cold () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f0910749c8b in <rayon_core::registry::ThreadBuilder>::run () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09107468ca in std::sys::backtrace::__rust_begin_short_backtrace::<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f091074ccfd in <std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#9  0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 2 (Thread 0x7f0901df66c0 (LWP 25421) "enomoto-2"):
#0  0x00007f09112ae256 in __GI___libc_realloc (oldmem=0x7f08d3014940, bytes=8192) at ./malloc/malloc.c:3456
#1  0x00007f09103fabbe in <alloc::raw_vec::RawVecInner>::finish_grow () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f09103f888a in <alloc::raw_vec::RawVec<u32>>::grow_one () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f090fff9e8d in <_core::mip::domain::Domain>::add_implications () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09101a9b0c in _core::mip::probe () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f09101907ba in _core::mip::presolve_mip_depth () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f0910176c09 in _core::mip::fj_p3_problem () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#7  0x00007f09103cdf0d in std::sys::backtrace::__rust_begin_short_backtrace::<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#8  0x00007f0910397646 in <std::thread::lifecycle::spawn_unchecked<<_core::mip::solver::Solver<_core::simplex::mip_lp::TwoStageLp>>::run::{closure#20}, core::option::Option<alloc::vec::Vec<f64>>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0} () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#9  0x00007f0910793e0f in <std::sys::thread::unix::Thread>::new::thread_start () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#10 0x00007f091129cb84 in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:447
#11 0x00007f0911329ecc in clone3 () at ../sysdeps/unix/sysv/linux/x86_64/clone3.S:78

Thread 1 (Thread 0x7f09115ab740 (LWP 24035) "python3"):
#0  syscall () at ../sysdeps/unix/sysv/linux/x86_64/syscall.S:38
#1  0x00007f09107779c5 in <std::sys::sync::condvar::futex::Condvar>::wait () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#2  0x00007f091074ce79 in <rayon_core::latch::LockLatch>::wait_and_reset () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#3  0x00007f091037ac68 in <std::thread::local::LocalKey<rayon_core::latch::LockLatch>>::with::<<rayon_core::registry::Registry>::in_worker_cold<<rayon_core::thread_pool::ThreadPool>::install<<_core::model::PyModel>::solve::{closure#1}::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult>::{closure#0}, _core::types::SolveResult> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#4  0x00007f09103a183a in <_core::model::PyModel>::__pymethod_solve__ () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#5  0x00007f0910392d0d in pyo3::impl_::trampoline::trampoline::<pyo3::impl_::trampoline::fastcall_with_keywords::{closure#0}, *mut pyo3_ffi::object::PyObject> () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
#6  0x00007f09103af2a0 in <pyo3::impl_::pyclass::PyClassImplCollector<_core::model::PyModel> as pyo3::impl_::pyclass::PyMethods<_core::model::PyModel>>::py_methods::ITEMS::trampoline () from /usr/local/lib/python3.13/site-packages/enomoto_solver/_core.cpython-313-x86_64-linux-gnu.so
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
[Inferior 1 (process 24035) detached]
