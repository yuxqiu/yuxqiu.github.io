---
layout: post
title: "Memcpy and Undefined Behavior"
date: 2022-12-05 09:05:17
tags: ["cpp"]

toc: true
featured: false
---

## Introduction

The aim of this article is to provide an axiomatic way of understanding `memcpy` and the undefined behaviour associated with it.

Before talking about the `memcpy`, it's necessary to understand
- some basics about C++ Standard Library
- C++ class layout
- Strict Aliasing


## C++ Standard Library

> \[defns.undefined\]: undefined behavior is behavior for which this document imposes no requirements

> \[library.c\]: The descriptions of many library functions rely on the C standard library for the semantics of those functions. In some cases, the signatures specified in this document may be different from the signatures in the C standard library, and additional overloads may be declared in this document, but the behavior and the preconditions (including any preconditions implied by the use of an ISO C restrict qualifier) are the same unless otherwise stated.


## Class Layout

> [Trivial, standard-layout, POD, and literal types](https://learn.microsoft.com/en-us/cpp/cpp/trivial-standard-layout-and-pod-types?view=msvc-170)

1. Trivial Types
   1. Definition
      1. no virtual functions or virtual base classes,
      2. no base classes with a corresponding non-trivial constructor/operator/destructor
      3. no data members of class type with a corresponding non-trivial constructor/operator/destructor
   2. Notes
      1. Trivial Types can have different access modifiers
      2. The C++ Standard places no requirements on the layout between different class modifiers. Compilers can feel free to optimize these (e.g. move private members to lower memory address even though they are declared after public members)
      3. [Potential optimizations that can be done](https://stackoverflow.com/a/52745420)

2. Standard layout types
   1. no virtual functions or virtual base classes
   2. all non-static data members have the same access control
   3. all non-static members of class type are standard-layout
   4. any base classes are standard-layout
   5. has no base classes of the same type as the first non-static data member.
   6. meets one of these conditions:
      1. no non-static data member in the most-derived class and no more than one base class with non-static data members, or
      2. has no base classes with non-static data members

3. POD
   1. Both Trivial and Standard Layout


## Strict Aliasing

> [Strict Aliasing](https://en.cppreference.com/w/cpp/language/object#Strict_aliasing)
>
> [Type aliasing](https://en.cppreference.com/w/cpp/language/reinterpret_cast#Type_aliasing)

Whenever an attempt is made to **read or modify** the stored value of an object of type DynamicType through a glvalue of type AliasedType, the behavior is undefined unless one of the following is true:
- AliasedType and DynamicType are [similar](https://en.cppreference.com/w/cpp/language/reinterpret_cast#Type_aliasing).
- AliasedType is the (possibly cv-qualified) signed or unsigned variant of DynamicType.
- AliasedType is std::byte, (since C++17) char, or unsigned char: this permits examination of the object representation of any object as an array of bytes.

More about Strict Aliasing
- [An intuitive way to understand strict aliasing](https://stackoverflow.com/a/98702)
- [Casting to void* and Casting back is perfectly defined](https://stackoverflow.com/a/18929285)


## Trap Representation

1. [What is a trap representation](https://stackoverflow.com/questions/6725809/trap-representation/6725981#6725981)
2. [What happens if we read a trap based on its type](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n2091.htm#problem)
   1. This applies as C++ (at least after 17) uses C11 as its normative references [intro.refs]
3. [Guarantee provided by C++ standard](https://en.cppreference.com/w/cpp/language/object#Object_representation_and_value_representation)
   1. This is the best justification I can find: [basic.fundamental.7] Type char is a distinct type that has an implementation-defined choice of “signed char” or “unsigned char” as its underlying type. The values of type char can represent distinct codes for all members of the implementation’s basic character set. The three types char, signed char, and unsigned char are collectively called ordinary character types. The ordinary character types and `char8_t` are collectively called narrow character types. For narrow character types, each possible bit pattern of the object representation represents a distinct value.


## Memcpy

So, what does the standard say about memcpy exactly? The standard mentions `std::memcpy` multiple times. But the quotes that cause many confusions is in \[basic.types\]:

> For any object (other than a base-class subobject) of trivially copyable type T, whether or not the object holds a valid value of type T, the underlying bytes (6.6.1) making up the object can be copied into an array of char, unsigned char, or std::byte (21.2.1). If the content of that array is copied back into the object, the object shall subsequently hold its original value. Example:
>
> ```c++
> #define N sizeof(T)
> char buf[N];
> T obj; // obj initialized to its original value
> std::memcpy(buf, &obj, N); // between these two calls to std::memcpy, obj might be modified
> std::memcpy(&obj, buf, N); // at this point, each subobject of obj of scalar type holds its original value
> ```
>
> For any trivially copyable type T, if two pointers to T point to distinct T objects obj1 and obj2, where neither obj1 nor obj2 is a base-class subobject, if the underlying bytes (6.6.1) making up obj1 are copied into obj2, obj2 shall subsequently hold the same value as obj1. Example:
>
> ```c++
> T* t1p;
> T* t2p;
> // provided that t2p points to an initialized object ...
> std::memcpy(t1p, t2p, sizeof(T));
> // at this point, every subobject of trivially copyable type in *t1p contains
> // the same value as the corresponding subobject in *t2p
> ```

Here, the standard lists two ways that are perfectly defined: we can copy to buffers of narrow character type and restore the object, or we can directly copy into another object of the same type. Some people believe that based on "[defns.undefined]", other uses of `memcpy` are not properly defined.

However, this is the time we need to directly refer to the `memcpy`. The C++ standard states that
- If the objects are potentially-overlapping or not TriviallyCopyable, the behavior of `memcpy` is not specified and may be undefined.[^1]
- Plus all the things stated by C standard (My Point)
  - Based on "[library.c]"

[^1]: [std::memcpy](https://en.cppreference.com/w/cpp/string/byte/memcpy)


So far, it looks like we have solid grounds to use `memcpy` in C++ as long as we are manipulating Trivially Copyable Types. However, some people mention about the trap, claiming that it's undefined behavior if we try to access the value based on the type that has trap representation. So, they believe it's not safe to copy from a type `T` to a type `U` unless `U` is of narrow character types or `U == T` (given that the destination has enough size for the src).[^2]

[^2]: [Is std::memcpy between different trivially copyable types undefined behavior?](https://stackoverflow.com/questions/51300626/is-stdmemcpy-between-different-trivially-copyable-types-undefined-behavior/)


However, my point is `memcpy` is noy affected by the existence of trap representation because as the standard explicitly stated in `std::memcpy`: "both objects (the src and dest pointers) are reinterpreted as arrays of unsigned char." Since we are not accessing the object of Type `U` via the type `U` itself, we are in the safe realm.

So, in summary, given the requirements about "size", "not overlapping", "trivially copyable", we can:
- Copy some bytes into some buffer (don't necessarily need to be narrow character types or the types of the src object)
- Restore an object by copying from some buffer to the storage of the object

---

Note-1: `memcpy` for Standard Layout Types:
- [Why is the behavior of memcpy unspecified, and probably undefined for standard layout types - 1](https://stackoverflow.com/questions/52871481/can-i-memcpy-objects-data-which-has-standard-layout-compatible-class-in-c11)
- [Why is the behavior of memcpy unspecified, and probably undefined for standard layout types - 2](https://stackoverflow.com/questions/29777492/why-would-the-behavior-of-stdmemcpy-be-undefined-for-objects-that-are-not-triv)


## Some discussions about Union

1. [Union is Undefined Behavior](https://adriann.github.io/undefined_behavior.html)
2. [Accessing inactive union member and undefined behavior?](https://stackoverflow.com/questions/11373203/accessing-inactive-union-member-and-undefined-behavior)


## Open Questions

- Are undefined behaviors always bad? Why don't we make some of UB as implementations defined behaviors?
- How should we interpret the C++ standard?