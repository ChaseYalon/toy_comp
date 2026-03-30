-- THIS IS A WORKING DOC!!! API, IMPLEMENTATION OR JUST ABOUT ANYTHING ELSE HERE SUBJECT TO CHANGE!!!!! --

# The Users Sees

### Option A
New Keywords: thread, await, and atomic
If parameters are not int, float, bool type, user can now edit the value and the caller will see them. All threaded functions must have the return type of HANDLE really an alias for c_int64_t which can then be used by the join keyword. There will also be a new function added to std.sys called kill(thread: HANDLE). Ex
```toy
thread fn foo(): HANDLE{
    println("hello world");
    //NO EXPLICIT RETURN!!!!!
}
thread fn fee(input: str): HANDLE{
    input = "hello";
}
let s = "hi";
let t1 = fee(s);
join t1;
println(s); //prints "hello"
let t2 = foo();
//explicit join not needed, programs will just continue independently
```
Atomics are more complicated but they allow TOCTOU conditions to be prevented. The way it works is that any code in an ```atomic{} ``` block is treated as a single transaction. absolutely no FFI access is allowed in atomic blocks, this eliminates all IO and threading stuff. cross_thread_read() and cross_thread_write() obviously have to be excepted. The compiler will then record in a memory buffer all reads and writes to the social heap. If a value that the atomic block read is overwritten, the whole block repeats. This should have exponential backoff, maybe add a user defined ceiling or a value (y=a^x) also.

# The Compiler Adds
1. Unique thread handles assigned by the runtime via the return toy_thread_uniq_handle() function.
2. cross_thread_write(c_void_ptr val, c_int64_t size): c_int64_t - writes the cross thread value into a social heap. Receives back a unique cross reference id from the runtime
3. cross_thread_read(c_void_ptr val): c_ToyPtr - simply asks the runtime for the value at that pointer in the social heap. It will always be exactly 64 bits, either a value or a pointer.
4. thread_register(c_int64_t alloc_id, c_int64_t thread_id): void - registers the thread as needing a value with the runtime. Threads may not access a value before they have registered for it.
5. seal_value(c_int64_t alloc_id): void - tells the runtime that no more threads will register for this value, and it may be freed when all threads have released it
6. thread_release(c_int64_t alloc_id, c_int64_t thread_id): void - tells the runtime that this thread no longer needs the given allocation.
7. thread_spawn(c_void_ptr func_ptr, c_void_ptr handle_ptr) - assigns the 64 bit handle to that pointer, and THEN spawns the thread at the function pointer requested.
8. thread_join(c_int64_t id) - halts current thread execution until the thread with the specified id is dead
9. thread_kill(c_int64_t id) - kills the provided thread
10. tx_begin(): c_int64_t — creates a transaction record, returns a transaction ID
11. tx_read(c_int64_t tx_id, c_int64_t alloc_id): c_ToyPtr — reads a value and logs it to the transaction's read set
12. tx_write(c_int64_t tx_id, c_int64_t alloc_id, c_ToyPtr val) — stages a write to the transaction's write buffer
13. tx_commit(c_int64_t tx_id): bool — attempts to commit, returns false if a conflict was detected and the block should retry

# The Runtime Does
Every cross thread object, ie an object in the social heap will have its ptr -> alloc_id and alloc_id -> ptr stored in a pair of hash maps. Each, in addition to their 64 bit pointer which should NOT change, will have a 64 bit counter, indicating the number of threads that are currently using that value. Whenever a thread calls thread_release it is decremented. The high bit is used to store if it is locked, by default all allocations are locked until seal_value is called, indicating that the value is no longer going to be taken by any thread. When the lower 63 bits reach 0, the value may be freed. Because both the counter and the seal bit are in one 64 bit chunk, it only needs one atomic for both of them, reducing overhead. Each value should also have a pthread RWLock allowing infinitely many threads to read, but only one thread to write. Every transaction needs a read write log. Atomics are really hard and should be done last, because thread_kill could also cause permanent locks unless handled carefully.