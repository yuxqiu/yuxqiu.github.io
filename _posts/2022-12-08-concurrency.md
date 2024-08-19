---
layout: post
title: "Concurrency"
date: 2022-12-08 20:39:06
tags: ["concurrency", "cpp", "java"]

toc: true
mathjax: false
featured: false
---

## Java Memory Model

For an object to be safely shared across different threads (visibility), it needs to be
- immutable, which JVM guarantees all the fields initialized before constructor exits
- effective immutable + safely published
- mutable + safely published + synchronization mechanism

### Immutable Object and Safe Publishing

1. Immutable = all the fields are marked with `final`

2. Safely published means
   1. The object is statically initialized
   2. The reference of the object is put into `final`
   3. The reference of the object is put into `volatile`: all the writes to the object directly writes to the memory instead of staying in cache
   4. Storing a reference to it into a field that is properly guarded by a lock


## Reordering (Java and C++)

1. Instruction reordering is a useful technique for Instruction Level Parallelism. However, it causes problems when we are sharing multiple variables during concurrency.
   1. The classic example is shown below:

        ```java
        boolean flag = false;
        int val = 0;

        void thread1(){
            val = 42;
            flag = true;
        }

        void thread2(){
            while(!flag){}
            System.out.println(val);
        }
        ```

   2. It's possible that the val printed is not equal to 42 because the instruction can be reordered such that flag is set to true before val is set to 42

2. To address this in Java, we could use `volatile` on flag. It gives a "happens-before" guarantee in addition to the visibility guarantee. The happens-before guarantee guarantees that: [^reordering-1]
   1. **Reads from and writes to other variables** cannot be reordered to occur after a write to a volatile variable, if the reads / writes originally occurred before the write to the volatile variable.
   2. The reads / writes before a write to a volatile variable are guaranteed to "happen before" the write to the volatile variable. Notice that it is still possible for e.g. reads / writes of other variables located after a write to a volatile to be reordered to occur before that write to the volatile. Just not the other way around. From after to before is allowed, but from before to after is not allowed.
   3. **Reads from and writes to other variables cannot be reordered to occur before a read of a volatile variable**, if the reads / writes originally occurred after the read of the volatile variable. Notice that it is possible for reads of other variables that occur before the read of a volatile variable can be reordered to occur after the read of the volatile. Just not the other way around.

3. C++'s `volatile` is different in terms of its guarantee. It says "volatile accesses cannot be optimized out or reordered with another visible side effect that is sequenced-before or sequenced-after the volatile access." [^reordering-2]
   1. [What is a visible side effect?](https://en.cppreference.com/w/cpp/language/as_if#Explanation)
   2. [What is sequenced-before?](https://en.cppreference.com/w/cpp/language/eval_order#Rules)
   3. So, consider the Java example in C++ context. We have different possibilities when we apply `volatile` to these two variables, see [compiler explorer](https://gcc.godbolt.org/z/hYM96r9r6) (Notice, C++ `volatile` cannot be used to address the synchronization problem in this example.)
      1. If both variables are non-volatile, they can be reordered
      2. If one of them are volatile, they can still be reordered (as the writes to another variable is not a visible side effect)
      3. If two of them are volatile, the order must be respected based on sequence-before rules

[^reordering-1]: https://jenkov.com/tutorials/java-concurrency/volatile.html "Java Volatile Keyword"
[^reordering-2]: https://en.cppreference.com/w/cpp/language/cv "cv (const and volatile) type qualifiers"

## Execution

### Execution within one thread

1. Java: "Each action in a thread happens-before every action in that thread that comes later in the program's order." [^execution-1]
   1. > It should be noted that the presence of a happens-before relationship between two actions does not necessarily imply that they have to take place in that order in an implementation. If the reordering produces results consistent with a legal execution, it is not illegal. [^execution-2]
2. C++: "Each value computation and side effect of a full-expression is sequenced before each value computation and side effect of the next full-expression." [^execution-3]
   1. as-if rule permits optimization. Those guarantees are the guarantees of C++ Abstract Machine.


[^execution-1]: https://docs.oracle.com/javase/8/docs/api/java/util/concurrent/package-summary.html#MemoryVisibility "Package java.util.concurrent: Memory Consistency Properties"
[^execution-2]: https://stackoverflow.com/a/32492873 "JLS happens before"
[^execution-3]: https://en.cppreference.com/w/cpp/language/as_if#Explanation "Order of evaluation: \"Sequenced before\" rules"


### Execution with multiple threads

1. `volatile` in Java can be used to ensure safe publishing
2. `volatile` in C++
   1. > Within a thread of execution, accesses (reads and writes) through volatile glvalues cannot be reordered past observable side-effects (including other volatile accesses) that are sequenced-before or sequenced-after within the same thread, but this order is not guaranteed to be observed by another thread, since volatile access does not establish inter-thread synchronization. [^execution-4]
   2. It's mainly used to deal with signal handling
   3. It's not intended to use for inter-thread communication (sequence-before is used to describe execution within one thread). Inter-thread communication should consider the memory fence or mutex.
   4. See [guarantees not provided by volatile](https://wiki.sei.cmu.edu/confluence/display/c/CON02-C.+Do+not+use+volatile+as+a+synchronization+primitive) for more details


[^execution-4]: https://en.cppreference.com/w/cpp/atomic/memory_order#Relationship_with_volatile "std::memory_order: Relationship with volatile"


## More resources

1. [An example of unsafe publishing](https://stackoverflow.com/questions/51695962/does-object-construction-guarantee-in-practice-that-all-threads-see-non-final-fi)
2. [Why unsafe publishing can occur?](https://stackoverflow.com/questions/16107683/improper-publication-of-java-object-reference)
3. [When to use volatile with multi threading?](https://stackoverflow.com/questions/4557979/when-to-use-volatile-with-multi-threading)