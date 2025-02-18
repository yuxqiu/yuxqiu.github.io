---
layout: post
title: 'Notes: Value Category in C++'
date: 2022-10-28 17:03:39
tags: ['cpp']

toc: true
featured: false
---

## How does C++ return a class

> Source: https://stackoverflow.com/questions/41910764/where-is-the-return-object-stored

Usually, allocated spaces are on the caller's stack. But this varies across ABI. So, in C++, compiling using the same compiler is the only way to avoid the ABI problem. C has a relatively more consistent ABI across different compilers.

## Understanding xvalue and prvalue

> Talk: https://www.youtube.com/watch?v=km3Urog_wzk
>
> PPT: [Kris-van-Rens-Understanding-Value-Categories.pdf](http://becpp.org/blog/wp-content/uploads/2021/07/Kris-van-Rens-Understanding-Value-Categories.pdf)

1. Before C++11, xvalue and prvalue are similar: we could steal resources from both of them, and they could both bind by the rvalue reference.
    1. The famous diagram illustrates their differences

        ![](/assets/img/blog/2022/10/1667134809024.png)
2. Since C++17, we need to understand the differences between these two to understand how the standard enforces its rule of optimization (namely copy elision)
    1. Before C++17, some optimizations are not guaranteed, but most compilers will do them (implementation-defined).
    2. First thing to notice, the value category is about expressions instead of types (See C++ Abstract Machine)
3. Elisions
    1. C++17 added mandates to the standard, informally known as: “Guaranteed copy elision”, “Guaranteed return value optimization”, and “Copy evasion”.
    2. Where can elisions occur?
        1. In the initialization of an object
        2. In a return statement
        3. In a throw expression
        4. In a catch clause
    3. Under the hood: how do we guarantee elision?
        1. Under the rules of C++17, a prvalue will be used only as an unmaterialized recipe of an object until actual materialization is required.
    4. Temporary materialization (cases where no elision happens)
	5. A prvalue is an expression whose evaluation initializes/materializes an object
        1. prvalues are not moved from as they are not materialized
        2. Why temporary materialization: we want to move resources of some un-materialized entity.
        3. Without materialization, we can feel free to construct the class in place in memory instead of calling copy/move constructors
        4. Temporary materialization occurs when
            1. Accessing a member of a prvalue
            2. Binding a reference to a prvalue (this is how `T &&` help us steal the resources)
            3. Applying sizeof or typeid to a prvalue

                ```c++
                struct Person {
                    std::string name_;
                    unsigned int age_ = {};
                };

                Person createPerson() {
                    std::string name;
                    unsigned int age = 0;
                    // Get data from somewhere in runtime
                    return Person{name, age}; // 1. Initial prvalue expression
                }

                int main() {
                    return createPerson().age_; // 2. Temporary materialization: xvalue
                }
                ```

## Return Value Optimization

> RVO and NRVO are unofficial terms used by people.

1. URVO/RVO
    1. Refers to the returning of temporary objects from a function. Guarantee to be optimized by C++17 rules.
2. NRVO
    1. Refers to the returning of named objects from a function.
    2. Have different cases
        1. Return named variable inside a function without conditions => YES
        2. Return static variable => NO
        3. Return the named variable inside a function with conditions, and the returned values are of different names => NO (the compiler doesn't know how to put them into the returned space)
        4. Return subclasses => NO (cannot be put into the returned space, must be copied or moved)
        5. Return arguments => NO (arguments are already in their position. whether or not a function decides to do copy elision is the function's decision, the caller doesn't need to know this -- think about the translational unit)
    3. In summary, RVO does not work when there’s no control over the physical location of the object.
    4. When even NRVO is not possible, it will cause an implicit move
        1. Sometimes, if NRVO is possible, DO NOT use a std::move in the return because this disables the NRVO

            ```c++
            T noNRVO()
            {
                T result;
                return std::move(result);
            }

            T NRVO(){
                T result;
                return result;
            }
```

## Differences between xvalue and prvalue when using `auto`

> Source: https://stackoverflow.com/questions/28641199/does-xvalue-prvalue-category-really-matter

xvalue and prvalue matter when using decltype deduction rules, for example. What cppreference says:
a) if the value category of expression is xvalue, then decltype yields T&&;
b) if the value category of expression is an lvalue, then decltype yields T&;
c) if the value category of expression is prvalue, then decltype yields T.

In this way, the compiler has a better hint of what kind of expression is coming. And of course, it's just important in a semantic sense of dealing with expressions.

```c++
struct A {};
int main() {
    decltype(auto) a1 = A(); //prvalue, type of a1 is A
    decltype(auto) a2 = std::move(A()); //xvalue, type of a2 is A&&
}
```