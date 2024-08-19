---
layout: post
title: "Implement a ThreadPool in C++ from Scratch"
date: 2023-01-13 18:20:23
tags: ["cpp", "concurrency"]

toc: true
mathjax: false
featured: false
---

## Prerequisite

Before reading this article, make sure you have basic understandings of the following concepts:
1. `std::thread`: how to create a thread, how to deal with a running thread, how the parameters are used in thread constructor
2. `std::mutex`, `std::lock_guard`, `std::unique_lock`: the differences between those three and the benefits of the later two
3. `std::condition_variable`:
4. `std::future`, `std::promise`: the relation between a `future` and a `promise`, how to construct them, can they be copied/moved constructed/assigned, the mechanisms behind `std::future` and `std::promise`
5. `std::function`: what is it, mechanisms of type erasure
6. `std::move`, `std::forward`: move semantics, universal reference, perfect forwarding
7. meta-programming: understand `if constexpr`, have basic ideas of type_traits


## Overview

Last night, I read an article in which a guy talked about how to implement a ThreadPool in C++. However, I wasn't happy with his implementation because he used some unnecessary dynamic memory allocation: 1 for `package_task` with `shared_ptr`, 1 for the mechanisms behind `std::future` (unavoidable), 2 for a list of threads and a list of tasks (unavoidable), 1 for every `std::function` used (unavoidable) and etc. The one that I really want to get rid of is the allocation for `package_task`. Considering I have never made a thread pool in C++, I decide to write it today.

So my goal for this thread pool is:
- use dynamic memory allocation as little as possible
- the supported version of C++ needs to be as low as possible (ideally C++11)
- the implementation needs to be as simple as possible

And, for the thread pool, it should at least support these APIs:
- `ThreadPool(std::size_t n)`: construct `n` threads for the thread pool
- `Stop`: stops the thread pool immediately, regardless of whether there are any outstanding tasks in the queue.
- `Enqueue`: push a task into the task list
- `~ThreadPool`: if the thread pool is not stopped, call `Stop`
- Cannot be copied/moved constructed/assigned

My ideas about how to implement this thread pool:
1. a thread pool with the specified number of threads is initialized
2. these threads will wait (mutex, condition_variable) until a task appears in the queue
   1. if notified, execute the task
   2. acquire a new task after finishing
   3. if there is no more task, wait
3. users can enqueue task into the list (mutex, function, promise, future, lambda)
   1. user's function will be encapsulated with parameters together to form a new callable
   2. the callable will be stored inside `std::function`


## Challenges

However, when I started to implement it, many problems suddenly appeared.

### Get Return Type of a Callable

The first question is how to correctly resolve the return type?

First, we need a return type for the `Enqueue` function since we need to have a template argument for `std::future`. We can easily solve this by using automatic return type deduction introduced in C++14 (just use `auto` for the return type).

Then, as we are dealing with a callable type, which can be a lambda, a function pointer or an object with overloaded `operator()`, we need a generic way to get its return type. That's the use case of `std::invoke_result`. By passing the type of the callable and the type of the arguments, we can easily get the returned type from it. This is introduced since C++17.

```c++
// an example of std::invoke_result
template<typename Callable, typename... Args>
auto f(Callable&& func, Args&& ...args){
    using ReturnType = typename std::invoke_result<F, Args...>::type;
    // other code
}
```

### Variadic Lambda?

After solving the previous problem, we get another one: how to create a variadic lambda?

The reason we need a lambda is that we need to encapsulate the callable from the user inside our callable, so that we could set the value of the `promise` when the task is executed. However, in order to encapsulate the user's task inside our lambda, we need a lambda that can capture variadic parameters because the `Args` are variadic. Fortunately, this is a feature that is introduced in C++20.

```c++
// an example of variadic lambda
template<typename Callable, typename... Args>
auto f(Callable&& func, Args&& ...args){
    auto a_new_callable = [...args = std::forward<Args>(args)](){
        // code inside
    }
}
```

However, it seems that we have gone too far from our second goal. Do we really need a lambda that captures variadic arguments?

Since eventually, we need to store all the parameters into our task object (otherwise, we may not be able to support temporary value and may access objects after their lifetime), it's reasonable to use `std::bind` instead of a lambda that captures variadic arguments. Also, it's worth mentioning that `std::bind` is introduced in C++11.

So, we finally have a solution:

```c++
template<typename Callable, typename... Args>
auto f(Callable&& func, Args&& ...args){
    auto a_new_callable = std::bind(std::forward<F>(func), std::forward<Args>(args)...);
}
```


### NOLINT

As I am using clang-tidy with some very "harsh" rules, it immediately gives me a warning for the use of `std::bind`. It reminds me to use lambda instead of `std::bind`. So, I need a way to bypass the linter. That's the usage of `NOLINT`.

```c++
auto a_new_callable = std::bind(std::forward<F>(func), std::forward<Args>(args)...); // NOLINT
```


### move-only-function

Things don't go well as expected. It seems that I understand why people use an additional dynamic memory allocation for the `std::promise` or the `std::package_task`. It's because these two types are not copyable.

The fact that they are not copyable causes a huge problem when we want to construct a `std::function` by using a lambda that captures them. Because they are not copyable, the resulting lambda is also not copyable. However, `std::function` is supposed to be copyable, and it has a copy constructor. Therefore, we fail to instantiate the template.

The solution to this problem is to use something called `std::move_only_function` introduced in C++23 (Thanks Jason Turner! I know this because I watched your video). Compared with the `std::function`, `std::move_only_function`, as its name suggests, can only be moved. Therefore, we could safely capture movable object in our lambda.

However, it's a C++23 feature, and I don't want the user of my thread pool to install a `trunk` compiler just for the `std::move_only_function`. So, I decide to provide a very naive implementation for `std::move_only_function`. But people who can use `c++2b` should definitely use their `std::move_only_function`. Therefore, I decide to use feature-test macro to help me do this.

```c++
#ifdef __cpp_lib_move_only_function
  template <typename Signature>
  using MoveOnlyFunction = std::move_only_function<Signature>;
#else
  // My MoveOnlyFunction here
#endif
```

To understand how to implement the `std::move_only_function`, I recommend you to watch this great talk [Back to Basics: Type Erasure - Arthur O'Dwyer - CppCon 2019](https://www.youtube.com/watch?v=tbUCHifyT24) by Arthur. You will learn what affordances are, how to implement hand-crafted type-erased classes, and how to rely on the v-table mechanism to implement type-erased classes.

This is my naive implementation of `std::move_only_function`:

```c++
template <typename Signature> class MoveOnlyFunction;

template <typename Ret, typename... Args>
class MoveOnlyFunction<Ret(Args...)> {
private:
class Base {
public:
    virtual ~Base() = default;
    virtual auto operator()(Args &&...args) -> Ret;
};

template <typename Callable> class Derived : public Base {
private:
    Callable func_;

public:
    explicit Derived(Callable &&func) : func_(std::forward<Callable>(func)) {}
    auto operator()(Args &&...args) -> Ret final {
    func_(std::forward<Args>(args)...);
    }
};

std::unique_ptr<Base> func_impl_;

public:
explicit MoveOnlyFunction() = default;

template <typename Callable>
explicit MoveOnlyFunction(Callable &&func) // NOLINT
    : func_impl_(std::make_unique<Derived<Callable>>(
            std::forward<Callable>(func))) {}

MoveOnlyFunction(const MoveOnlyFunction &) = delete;
auto operator=(const MoveOnlyFunction &) = delete;
MoveOnlyFunction(MoveOnlyFunction &&other) noexcept {
    func_impl_ = other.func_impl_;
    other.func_impl_ = nullptr;
}
auto operator=(MoveOnlyFunction &&other) noexcept -> MoveOnlyFunction & {
    if (this != &other) {
        std::swap(other.func_impl_, func_impl_);
    }
    return *this;
}

auto operator()(Args &&...args) -> Ret {
    (*func_impl_)(std::forward<Args>(args)...);
}
};
```

Do notice that there is a very interesting trick about taking `Signature` as a template argument here. I provide a partial specialization of the class to actually make the `Signature` trick work.


## The final code

The final implementation looks like this:

```c++
#include <condition_variable>
#include <deque>
#include <functional>
#include <future>
#include <memory>
#include <mutex>
#include <thread>
#include <type_traits>
#include <utility>
#include <vector>

class ThreadPool {
private:
#ifdef __cpp_lib_move_only_function
  template <typename Signature>
  using MoveOnlyFunction = std::move_only_function<Signature>;
#else
  // A simple implementation of MoveOnlyFunction
  template <typename Signature> class MoveOnlyFunction;

  template <typename Ret, typename... Args>
  class MoveOnlyFunction<Ret(Args...)> {
  private:
    class Base {
    public:
      virtual ~Base() = default;
      virtual auto operator()(Args &&...args) -> Ret;
    };

    template <typename Callable> class Derived : public Base {
    private:
      Callable func_;

    public:
      explicit Derived(Callable &&func) : func_(std::forward<Callable>(func)) {}
      auto operator()(Args &&...args) -> Ret final {
        func_(std::forward<Args>(args)...);
      }
    };

    std::unique_ptr<Base> func_impl_;

  public:
    explicit MoveOnlyFunction() = default;

    template <typename Callable>
    explicit MoveOnlyFunction(Callable &&func) // NOLINT
        : func_impl_(std::make_unique<Derived<Callable>>(
              std::forward<Callable>(func))) {}

    MoveOnlyFunction(const MoveOnlyFunction &) = delete;
    auto operator=(const MoveOnlyFunction &) = delete;
    MoveOnlyFunction(MoveOnlyFunction &&other) noexcept {
      func_impl_ = other.func_impl_;
      other.func_impl_ = nullptr;
    }
    auto operator=(MoveOnlyFunction &&other) noexcept -> MoveOnlyFunction & {
      if (this != &other) {
        std::swap(other.func_impl_, func_impl_);
      }
      return *this;
    }

    auto operator()(Args &&...args) -> Ret {
      (*func_impl_)(std::forward<Args>(args)...);
    }
  };
#endif

  std::deque<MoveOnlyFunction<void()>> tasks_{};
  std::vector<std::thread> threads_{};
  std::mutex lock_{};
  std::condition_variable cv_{};
  bool stopped_ = false;

public:
  explicit ThreadPool(std::size_t threads) {
    for (std::size_t i = 0; i < threads; ++i) {
      threads_.emplace_back([this]() {
        while (true) {
          auto task = MoveOnlyFunction<void()>{}; // benchmark inside or outside
          {
            auto ulock = std::unique_lock<std::mutex>{lock_};
            cv_.wait(ulock, [this]() {
              return this->stopped_ || !this->tasks_.empty();
            });

            if (stopped_) {
              return;
            }

            task = std::move(tasks_.front());
            tasks_.pop_front();
          }
          task();
        }
      });
    }
  }
  ~ThreadPool() {
    if (!stopped_) {
      Stop();
    }
  }
  ThreadPool(const ThreadPool &) = delete;
  ThreadPool(ThreadPool &&) = delete;
  auto operator=(ThreadPool) -> ThreadPool & = delete;

  auto Stop() -> void {
    stopped_ = true;
    cv_.notify_all();
    for (auto &thread : threads_) {
      thread.join();
    }
  }

  template <typename Callable, typename... Args>
  auto Enqueue(Callable &&func, Args &&...args) {
    using Return_Type = std::invoke_result_t<Callable, Args...>;

    auto promise = std::promise<Return_Type>{};
    const auto future = promise.get_future();
    {
      const auto guard = std::lock_guard<std::mutex>{lock_};
      tasks_.emplace_back(
          [promise = std::move(promise),
           task = std::bind(std::forward<Callable>(func),
                            std::forward<Args>(args)...)]() mutable {
            if constexpr (std::is_void_v<Return_Type>) {
              std::invoke(task);
              promise.set_value();
            } else {
              promise.set_value(std::invoke(task));
            }
          });
    }
    cv_.notify_one();
    return future;
  }
};
```


## Benchmark

I create a simple benchmark for my implementation. I compare it with one of the most famous [implementation](https://github.com/bshoshany/thread-pool) in Github.

The benchmark code is something like this:

```c++
#include <chrono>
#include <iostream>

class Timer {
public:
  Timer() : m_start_(std::chrono::high_resolution_clock::now()) {}
  ~Timer() { Stop(); }

private:
  void Stop() {
    const auto end_time = std::chrono::high_resolution_clock::now();
    const auto start =
        std::chrono::time_point_cast<std::chrono::milliseconds>(m_start_)
            .time_since_epoch()
            .count();
    const auto stop =
        std::chrono::time_point_cast<std::chrono::milliseconds>(end_time)
            .time_since_epoch()
            .count();
    const auto duration = stop - start;
    std::cout << "Duration: " << duration << " ms\n";
  }

  std::chrono::time_point<std::chrono::high_resolution_clock> m_start_;
};

static auto Worker(int size) -> int {
  int counter = 0;
  for (int i = 0; i < size; ++i) {
    if (i % 2 == 0) {
      ++counter;
    }
  }
  return counter;
}

static void ThreadPoolTest(std::size_t size) {
  // different thread pool is used here
  auto pool = ThreadPool{std::thread::hardware_concurrency()};
  Timer timer;

  std::vector<std::future<int>> result;
  result.reserve(size);
  for (std::size_t i = 0; i < size; ++i) {
    result.emplace_back(pool.Enqueue(Worker, 1000000));
  }

  for (auto &future : result) {
    future.wait();
  }
}

auto main() -> int {
  ThreadPoolTest(100);
  ThreadPoolTest(1000);
  ThreadPoolTest(10000);
  ThreadPoolTest(100000);
  ThreadPoolTest(1000000);
}
```

I have plotted a graph for the benchmark results and it looks very good. (All the code are compiled with `-Ofast`).

![benchmark](/assets/img/blog/2023/01/threadpool-benchmark.svg)

By using `hyperfine`, I am able to get the following result

```
Benchmark 1: ./bench (my thread pool)
  Time (mean ± σ):     11.814 s ±  0.366 s    [User: 81.934 s, System: 1.862 s]
  Range (min … max):   11.410 s … 12.335 s    10 runs

Benchmark 2: ./bench-bs (bs thread pool light)
  Time (mean ± σ):     12.680 s ±  0.061 s    [User: 88.338 s, System: 1.866 s]
  Range (min … max):   12.578 s … 12.766 s    10 runs

Summary
  './bench' ran
    1.07 ± 0.03 times faster than './bench-bs'
```

So, it's roughly 1.1 times faster than bs's implementation.


## Improvements

There are certainly many rooms for improvements.

1. One can argue that the API is far less than enough as it provides no way to inspect the number of tasks and threads. In addition, there is not enough customisability: the user cannot decide how to start the thread (in detached mode or joined mode).

2. The task queue can be implemented via some lock-free mechanisms. To further reduce the contention, we can even create a queue for every thread and implement work-stealing mechanism.

3. Possible optimizations of `MoveOnlyFunction`. For `std::string`, we have something called Small String Optimization (SSO). The same solution exists for `std::function`, which is known as Small Function Optimization (SFO). We can apply this to `MoveOnlyFunction` to save us one dynamic memory allocation for small callable objects.

4. Exception guarantee. Currently, we neglect the exception guarantee provided by the user. We should take this into account when packaging user's callable and arguments into a task.


## What I learned

1. `std::invoke_result`
2. lambda that captures variadic arguments
3. `std::move_only_function`
4. more about lambda captures
   1. [Lambda capture as const reference?](https://stackoverflow.com/questions/3772867/lambda-capture-as-const-reference)
5. type-erased class in practice
6. meta-programming in practice
   1. perfect forwarding
   2. `if constexpr`
   3. type traits