---
layout: post
title: 'Talking about strerror (and why you should be cautious when using it)'
date: 2022-09-15 20:15:48
tags: ['c','cpp']

toc: true
featured: false
---

If you have ever used C/C++ before, you must know a very important global variable `errno` (though it can be a macro). Through this variable, we can get information about the library function call (whether it succeeds? If it fails, what's the type of error?). However, `errno` is just a number, and we want to interpret its meaning. Thus, we need some functions for us to "interpret" it. That's the reason why `strerror` exists.

## What is `strerror`

Let's first take a look at the signature of the function.

`char* strerror( int errnum );`

The function takes in an integer `errnum` and "returns a pointer to the textual description of the system error code errnum"[^1].

[^1]: [cppreference.com: strerror, strerror_s, strerrorlen_s](https://en.cppreference.com/w/c/string/byte/strerror)

The description is very vague, but we do know
- We don't need to free the memory of the `char *` returned from this function
- We don't need to make sure `errnum` is in the valid range of `errno`

Let's read the rest of the description:
>The returned string must not be modified by the program but may be overwritten by a subsequent call to the `strerror` function. `strerror` is not required to be thread-safe. Implementations may be returning different pointers to static read-only string literals or may be returning the same pointer over and over, pointing at a static buffer in which `strerror` places the string.

So, why the string returned may be overwritten by subsequent calls? To better understand it, let's imagine you need to implement a function that maps some integers to error messages. What would you do? I believe most of you, including me, will define some `const char*` strings as messages and returns one of them to the user. As far as I know, most implementations simply return a `const char*` to the user.

So, where does the possible overwriting come from? Let's look at the signature again `char* strerror( int errnum );`. It's the `errnum`. Remember, we could pass anything that is an `int` to this function. What would happen if we pass an integer, not in the range of the `errno`? (Clearly, your system doesn't have that many error types).

Let's see what would happen with the following programs:

```c
#include <stdio.h>
#include <string.h>

int main(void){
    printf("%s\n", strerror(-1));
    return 0;
}

```

In my macOS Monterey 12.6, I get `Unknown error: -1`. You can see that it outputs the integer that we pass to the function.

If you are familiar with C, you may immediately realize how we could implement it: a static buffer inside this function with some predefined `const char*` message strings. If we have a valid `errnum`, we return one of the message strings; otherwise, we construct one inside our buffer and return the address of the buffer.

That's the root of evil.

## Solutions before C11

Imagine if you want to implement a multi-threaded web server. You use system functions to create a socket and read/write data. Suppose some bad things happen, and two of your threads need to use `strerror` to acquire the error messages. Now, you face this situation:
- You know most implementations return a `const char*` for valid `errno`
- However, in theory, "strerror is not required to be thread-safe" (because of the static buffer). So if some unwise people, when developing this function, copy the error message to a static buffer and return it, you will have no idea what error messages you will get.

So, how could we mitigate this (before C11) if we want a portable version of `strerror`?
1. Create a wrapper function and use a `mutex`. There are two possible implementations:
    1. Before releasing the `mutex`, copy the string into a dynamically allocated buffer. This method requires the user to free the returned result.
    2. Accept a user-provided buffer as a parameter and take in another parameter which is the size of the buffer. Copy the string into the buffer based on the given size.
2. You could use `sys_nerr` and `sys_errlist`. The first argument is the length of the `sys_errlist`, and the second is an array that stores pointers to error messages. The detailed usage is not provided here because it is "deprecated and declared inconsistently" in some systems[^2].
3. Use `strerror_r` (which is another evil function)

[^2]: [Mac OS X manual page for strerror(3)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/strerror.3.html)

## The evilness of `strerror_r`

`strerror_r` is defined by POSIX as `int strerror_r(int errnum, char *buf, size_t buflen);`.

>However, it has a different, non-standard prototype in glibc versus in POSIX: `char *strerror_r(int errnum, char *buf, size_t buflen);`
Even if you don't define _GNU_SOURCE or any other of the feature macros, you'll still get the non-POSIX definition on Linux.
Ok, you say, no big problem. I don't expect strerror_r to fail, so I should be able to get by with code like this:

```c
strerror_r(errnum, buf, sizeof(buf)); // do NOT do this!
printf("foobar failed: %s\n", buf);
```

>But, as the comment indicates, you must not do this. The GNU function doesn't usually modify the provided buffer-- it only modifies it sometimes. If you end up with the GNU version, you must use the return value of the function as the string to print[^3].

[^3]: [2012-08-12: A portable strerror_r](http://www.club.cc.cmu.edu/~cmccabe/blog_strerror.html)

## Solution after C11

We finally see some great news since C11. We now have a improvded function in C standards: `errno_t strerror_s( char *buf, rsize_t bufsz, errno_t errnum );`

>...The message is copied into user-provided storage `buf`. No more than `bufsz-1` bytes are written, and the buffer is always null-terminated. If the message had to be truncated to fit the buffer and `bufsz` is greater than 3, then only `bufsz-4` bytes are written, and the characters "..." are appended before the null terminator.

So, use this function if your project uses `C11` or above.

## What about C++?

Sadly, C++ doesn't have `strerror_s` or `strerror_r`. The only thing we have is `strerror`. So, create a wrapper and use a `mutex` if you care about "perfect portability".

Also, you should avoid using `<system_error>` (at least avoid using the error associated with `std::error_code`) for the same reason since its internal mechanism uses `strerror` to convert the system error code into the error message[^4].

[^4]: [libstdc++ implementation of system_error](https://github.com/gcc-mirror/gcc/blob/master/libstdc%2B%2B-v3/include/std/system_error)