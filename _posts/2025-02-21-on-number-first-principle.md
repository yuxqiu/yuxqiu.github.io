---
layout: post
title: "On Number Theory from First Principle"
date: 2025-02-21 12:37:28
tags: ["TIL", "Cryptography", "Math"]

featured: false
katex: true
---

Recently, I was reading [Number theory explained from first principles](https://explained-from-first-principles.com/number-theory/). It's a great article that covers many aspects of number theory that are useful in cryptography. This blog serves the purpose to additionally prove/clarify some aspects that I think could have been explained better in the original article.

## Even number of generators in cyclic group with more than 2 elements

> Cyclic groups, Even number of generators, pdf 11/92

**Prove: Every cyclic group with more than two elements has an even number of generators.**

---

To prove this, we first prove the following lemma.

> For a cyclic group with more two elements, all of its generators are not self-inverse.
{: .block-tip }

Consider a generator $$g$$ for this group. If it's self inverse, then $$gg = e$$. But, by assumption, the group has more than two elements. So, $$g$$ is not a generator, which means we have reached a contradiction.
{: .proof-end }

---

Now, let's consider the set $$S$$ of all generators of a cyclic group. We want to show that it has even number of elements.

Because of the lemma above, we know we can group elements in this set into pairs like $$(g_1, g_2)$$ where $$g_1$$ is the inverse of $$g_2$$.
- Here, we utilize the fact that the inverse of generator is also a generator. To prove this, it suffices to show that every element in the group can be expressed as a multiple/exponent of the inverse of the generator.

Thus, the number of elements in the group is a multiple of 2, which is an even number.
{: .proof-end }

## Even order element in $$Z_m^+$$

> Repetition table, pdf 17/92

**Prove: Whenever an element has an even order (which can be the case only if $$m$$ is even), you reach $$\frac{m}{2}$$ halfway to the identity element.**

---

Before proving this, it's important to identify the following lemma discussed in the text:

> An element $$a$$ has an order of 2 iff $$a = \frac{m}{2}$$.
{: .block-tip }

This can be proved by exhaustion: every element except $$\frac{m}{2}$$ has an order different from 2.
{: .proof-end }

---

Now, let's consider an element $$a$$ with even order $$q$$. Let's denote $$b = a^\frac{q}{2}$$.

Then, it's easy to see $$b^2 = a^q = e$$. Therefore, $$b$$ has an order of 2. By the lemma, $$b = \frac{m}{2}$$.
{: .proof-end }

## Difference between adjacent elements of the subgroup $$\langle a \rangle \leq Z_m^+$$

> Subgroup cosets, Visualization of cosets, pdf 18/92

**Prove: Difference between adjacent elements of the subgroup $$\langle a \rangle \leq Z_m^+$$ equals $$\text{gcd}(a, m).$$**

---

Let's first prove the following lemma:

> The differences between every adjacent pair $$(a, b) \in \langle a \rangle$$ are the same.
{: .block-tip }

Proof by contradiction. Consider two pairs of adjacent elements $$(a, b)$$ and $$(c, d)$$ such that $$d + -c < b + -a$$. Then, we can construct a new element $$m = a + d + -c$$. By closure, $$m \in \langle a \rangle$$.

By construction, $$m + -a = d + -c < b + -a$$. Therefore, $$m$$, instead of $$b$$, should be the adjacent element of $$a$$, which results in a contradiction.
{: .proof-end }

---

Now, let's complete the original proof.

Since the identity element $$0 \in \langle a \rangle$$, with the above lemma, it suffices to prove that the smallest positive element in $$\langle a \rangle$$ is $$\text{gcd}(a, m)$$.

Note that
- By definition, the smallest positive element in $$\langle a \rangle$$ can be written as $$ka + qm$$ for some $$k > 0, q < 0$$.
- By [Bézout’s identity](https://en.wikipedia.org/wiki/B%C3%A9zout%27s_identity), there exists $$x, y$$ such that $$xa + ym = \text{gcd}(a, m)$$ and $$\text{gcd}(a, m)$$ is the smallest positive integer that can be derived from the linear combination of $$a, m$$.

So, it suffices for us to show that there exists $$x', y'$$ such that $$x' > 0, y' < 0, x'a + y'm = \text{gcd}(a, m)$$.

Note that $$ma + (-a)m = 0$$. So, there exists some natural number $$z$$ such that $$x' = x+zm > 0$$ and $$y' = y - za < 0$$ such that $$(x+zm)a + (y-za)m = xa + ym + zma - zam = xa + ym = \text{gcd}(a, m)$$.
{: .proof-end }

## Latin square + associativity = group

> Identity row and column, Latin square + associativity = group, pdf 20/92

$$
\begin{array}{c|ccc}
\circ & A & B & C \\
\hline
A & B & A & C \\
B & A & C & B \\
C & C & B & A
\end{array}
$$

A 3x3 latin square.
{: .caption}


**Prove: Associative operation with unique solutions implies that the identity element is the same for all elements.** Another equivalent expression is: [latin square](https://en.wikipedia.org/wiki/Latin_square) + associative operation implies a group.

Note:
- The proof in the article is already very good. Here, I aim at simplifying it a bit.
- The uniqueness of solution results in a latin square is proven previously in the article.

Without loss of generality, we prove that the left identity element is the same for all elements.

---

Let's first prove the following lemma:

> The left identity element of each element is idempotent.
{: .block-tip }

Consider a left arbitrary element $$D$$. By definition of latin square, $$\exists E, E \cdot D = D$$. By applying $$E$$ on both side, we get $$E \cdot (E \cdot D) = E \cdot D$$. By associativity, $$E \cdot (E \cdot D) = (E \cdot E) \cdot D = E \cdot D = D$$. By uniqueness of the solution for $$X \cdot D = D$$, $$E \cdot E = E$$.
{: .proof-end }

---

Now, let's prove the left identity element of all elements is the same.

Consider a left identity element $$E$$ of $$D$$, and another arbitrary element $$F$$. By assumption, $$E \cdot X = F$$ has a unique solution. By idempotency, it follows that $$E \cdot X = (E \cdot E) \cdot X$$. By associativity, $$(E \cdot E) \cdot X = E \cdot (E \cdot X) = F$$. Plug in the value of $$E \cdot X$$, we get $$E \cdot F = F$$. So, $$E$$ is a left identity of $$F$$.
{: .proof-end }

## To be continued.