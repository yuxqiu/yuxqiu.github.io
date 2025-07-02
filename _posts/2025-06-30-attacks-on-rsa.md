---
layout: post
title: "Attacks on RSA"
date: 2025-06-30
tags: ["Cryptography"]

featured: false
katex: true
katex_macros:
  "\\tp": "\\textcolor{orange}{t_p}"
  "\\tq": "\\textcolor{blue}{t_q}"
---

> **tl;dr**: I discuss the issues related to small exponents, how to improve the success rate of an attack on RSA, how the square root relates to the hardness of the RSA problem, and its connection with Shor's algorithm. Many of the materials discussed below are thanks to the lecture notes and coursework that I have taken in [Introduction to Cryptography](https://www.ucl.ac.uk/module-catalogue/modules/introduction-to-cryptography-COMP0025).
{: .block-tip }

## Prerequisites

- [RSA](https://en.wikipedia.org/wiki/RSA_cryptosystem#Operation)
- [Chinese Remainder Theorem (CRT)](https://en.wikipedia.org/wiki/Chinese_remainder_theorem)
- [Shor's Algorithm](https://en.wikipedia.org/wiki/Shor%27s_algorithm) (for the last section only)

## Small Exponent Attack

Alice wants to send a (k − 1)-bit message, \( m \), to Bob, Cathy, and Dave. Bob, Cathy, and Dave have distinct k-bit RSA keys, \( N_B \), \( N_C \), and \( N_D \), and they all use the same exponent, \( e = 3 \). Alice uses plain textbook RSA to encrypt \( m \) under each key, resulting in three ciphertexts \( c_B \), \( c_C \), and \( c_D \), which she sends to them. Eve intercepts all three ciphertexts. Show that Eve can recover the plaintext \( m \).

---

There are two cases to consider:

1. **\( N_A \), \( N_B \), and \( N_C \) are not pairwise coprime**.

   Without loss of generality, suppose \( N_A \) and \( N_B \) are not coprime. Then one can use \( \gcd(N_A, N_B) \) to factor both \( N_A \) and \( N_B \). Once factored, computing \( \phi(N_A) \) is straightforward, which then enables decryption of the ciphertext \( m^e \bmod N_A \).

2. **\( N_A \), \( N_B \), and \( N_C \) are pairwise coprime**.

   In this case, we have:

   $$
   \begin{align*}
   c_A &= m^3 \bmod N_A \\
   c_B &= m^3 \bmod N_B \\
   c_C &= m^3 \bmod N_C \\
   \end{align*}
   $$

   Using the Chinese Remainder Theorem (CRT), we can compute a value \( c' \) such that

   $$
   c' = m^3 \bmod (N_A N_B N_C)
   $$

   Since \( m \) is smaller than any of the moduli, \( m^3 < N_A N_B N_C \). Therefore, the cube root of \( c' \) can be taken directly to recover \( m \).
   {: .proof-end }

## Improve RSA Attack Success Rate

Let \( (N, e) \) be an RSA key, where \( 2 < e < N \). Suppose \( \mathcal{A} \) is a probabilistic algorithm that runs in time \( t \) with

$$
\Pr_{r \leftarrow \mathbb{Z}_N} \left[ \mathcal{A}(N, e, r^e \bmod N) = r \right] = 1\%.
$$

Show that \( \mathcal{A} \) can be used as a subroutine to construct an algorithm \( \mathcal{B} \) such that

$$
\Pr_{r \leftarrow \mathbb{Z}_N} \left[ \mathcal{B}(N, e, r^e \bmod N) = r \right] = 99\%.
$$

---

To build such an algorithm \( \mathcal{B} \), we rely on the fact that [RSA is multiplicatively homomorphic](https://en.wikipedia.org/wiki/Homomorphic_encryption#Partially_homomorphic_cryptosystems). With this in mind, one can devise an algorithm \( \mathcal{B}(N, e, y) \) as follows:

- **Repeat:**
  - Sample \( s \xleftarrow{\$} \mathbb{Z}_N \).
  - Compute \( x \leftarrow \mathcal{A}(N, e, ys^e \bmod N) \).
- **Until** \( x^e = ys^e \).

Then, by using the homomorphic property, recover \( r \) as:

$$
r \leftarrow x \cdot s^{-1} \bmod N.
$$

By repeating the algorithm \( \mathcal{A} \) a bounded number of times, we can design \( \mathcal{B} \) with a success probability of 99%.
{: .proof-end }

## Square Root and RSA

Let's begin by examining an interesting attack.

### RSA Key Leakage Vulnerability

Suppose Bob uses the RSA cryptosystem and has a key pair with public key \( e \) and private key \( d \). If Bob accidentally leaks his private key and decides to generate a new public/private key pair without generating a new modulus, show that this approach is insecure.

---

To prove that this approach is insecure, we will show that, given \( d \), one can factor \( N \) (assuming \( N \) is the product of two odd primes).

1. Given \( d \), compute \( k = ed - 1 \) using the public key \( e \).
2. By definition, \( k \) is divisible by \( \phi(N) \). Since Euler's totient function \( \phi(N) \) is even, \( k \) is even, and hence \( \frac{k}{2} \) is an integer.
3. By Euler's theorem, for every \( g \in \mathbb{Z}_N^* \), we have \( g^k = 1 \). Therefore, \( g^{\frac{k}{2}} \) is a square root of \( 1 \bmod N \).

### Properties of Square Roots

Before proceeding with the proof, let's study the properties of the square roots of \( 1 \), which we denote by \( x \).

Using the CRT, since \( N = pq \) where \( p \) and \( q \) are primes, solving

$$
x^2 \equiv 1 \pmod{N}
$$

is equivalent to solving

$$
x^2 \equiv 1 \pmod{p} \quad \text{and} \quad x^2 \equiv 1 \pmod{q}.
$$

For a prime \( p \), the solutions to \( x^2 \equiv 1 \pmod{p} \) are \( x \equiv \pm 1 \pmod{p} \). Similarly for \( q \). Hence, there are four solutions modulo \( N \), determined by the combinations of \( \pm 1 \pmod{p} \) and \( \pm 1 \pmod{q} \):

1. \( n_1 \equiv 1 \pmod{p} \) and \( n_1 \equiv 1 \pmod{q} \), i.e., \( n_1 \equiv 1 \pmod{N} \).
2. \( n_2 \equiv -1 \pmod{p} \) and \( n_2 \equiv -1 \pmod{q} \), i.e., \( n_2 \equiv -1 \pmod{N} \).
3. \( n_3 \equiv 1 \pmod{p} \) and \( n_3 \equiv -1 \pmod{q} \).
4. \( n_4 \equiv -1 \pmod{p} \) and \( n_4 \equiv 1 \pmod{q} \).

Each \( n_i \) satisfies \( n_i^2 \equiv 1 \pmod{N} \) because \( n_i^2 \equiv 1 \) modulo both \( p \) and \( q \).

The nontrivial solutions \( n_3 \) and \( n_4 \) are especially important for factoring \( N \). Consider \( n_3 \):
- Since \( n_3 \equiv 1 \pmod{p} \), we have \( n_3 - 1 \equiv 0 \pmod{p} \); hence, \( p \) divides \( n_3 - 1 \).
- Since \( n_3 \equiv -1 \pmod{q} \), we have \( n_3 - 1 \equiv -2 \pmod{q} \), and since \( q \) is odd, \( -2 \not\equiv 0 \pmod{q} \). Thus, \( q \) does not divide \( n_3 - 1 \).

Therefore, \( \gcd(n_3 - 1, N) = p \). A similar argument applies to \( n_4 \), except it yields \( q \).

However, finding a nontrivial square root of 1 is not straightforward in RSA. In fact, it can be shown that for every \( g \in \mathbb{Z}_N^* \), \( g^{\frac{k}{2}} = 1 \bmod N \). To explain this, express \( p-1 = 2^{\tp} r_p \) and \( q-1 = 2^{\tq} r_q \) with \( \tp, \tq \geq 1 \). Then,

$$
k = \alpha \cdot 2^{\tp + \tq} r_p r_q,
$$

where \( \alpha \) is a positive integer and \( \tp + \tq \geq 2 \). This implies that \( p - 1 \) divides \( \frac{k}{2} \) and similarly for \( q - 1 \). By the CRT and Euler's Theorem, it follows that

$$
\forall g \in \mathbb{Z}_N^*, \; g^{\frac{k}{2}} \equiv 1 \bmod N.
$$

On the bright side, if \( \frac{k}{2} \) is even, we can iterate and consider \( g^{\frac{k}{4}} \), \( g^{\frac{k}{8}} \), and so on to search for a nontrivial square root of 1. Formally, writing \( k = 2^t r \) where \( t \geq \tp + \tq \) and \( r \) is an odd integer, we can examine the sequence:

$$
g^{\frac{k}{2}},\ g^{\frac{k}{4}},\ \dots,\ g^{\frac{k}{2^t}}.
$$

If, for any \( i \), the value \( \gcd\left(g^{\frac{k}{2^i}} - 1, N\right) \) is neither 1 nor \( N \), then we can successfully factor \( N \)[^1].

> But what is the probability that such an event occurs when we randomly select \( g \)?
{: .block-tip }

We will now prove a lower bound: **such an event occurs with probability at least \( \frac{1}{2} \)**. Let \( t_{\text{max}} = \max(\tp, \tq) \).

**A strawman example:** First, consider \( k' = 2^{t'} r \) where \( t' \geq t_{\text{max}} \). Since \( t' \geq t_{\text{max}} \), both \( p - 1 \) and \( q - 1 \) divide \( k' \). Thus, for any \( l \leq t - t_{\text{max}} \), \( g^{\frac{k}{2^l}} \) will always be 1. This suggests that we should instead look at the case when \( t' < t_{\text{max}} \).

---

Before proceeding further, we prove the following lemma:

> **Lemma**
> 1. The multiplicative group \( \mathbb{Z}_p^* \) (with \( p \) prime) is cyclic; that is, it has a generator.
> 2. \( \mathbb{Z}_p^* \) has a subgroup \( S \) of order \( \frac{p-1}{2} \), and for every \( e \in \mathbb{Z}_p^* \), we have \( e^{\frac{p-1}{2}} \equiv 1 \bmod p \) if and only if \( e \in S \).
{: .block-warning }

**Proof.**

1. This follows from the standard result that [the multiplicative group of a finite field is cyclic](https://math.stanford.edu/~conrad/210BPage/handouts/math210b-finite-mult-groups-cyclic.pdf). Let \( g \) denote a generator of this group.

2. The subgroup \( S \) exists and can be generated by \( g^2 \). Its order is \( \frac{p-1}{2} \) because \( \frac{p-1}{2} \) is the smallest integer satisfying \( \left(g^2\right)^{\frac{p-1}{2}} = g^{p-1} \equiv 1 \bmod p \). The rest of the lemma follows:
   - **($$\Leftarrow$$)** If \( e \in S \), then \( e = (g^2)^c \) for some integer \( c \), so

     $$
     e^{\frac{p-1}{2}} = \left(g^2\right)^{c \frac{p-1}{2}} = 1^c = 1 \bmod p.
     $$

   - **($$\Rightarrow$$)** Conversely, if \( e^{\frac{p-1}{2}} \equiv 1 \bmod p \) and \( e = g^c \), then

     $$
     g^{c \cdot \frac{p-1}{2}} \equiv 1 \bmod p.
     $$

     Since \( g \) has order \( p-1 \), it must be that \( c \cdot \frac{p-1}{2} \) is a multiple of \( p-1 \); hence, \( c \) is even. Writing \( c = 2k \), we have \( e = g^{2k} = (g^2)^k \), so \( e \in S \). This also implies that if \( e \notin S \), then \( e^{\frac{p-1}{2}} \equiv -1 \bmod p \).
     {: .proof-end }

---

Now, consider the case \( t' = t_{\text{max}} - 1 \). There are two cases to analyze:

1. **Case 1: \( \tp < \tq \) (without loss of generality).**

   Since \( t_{\text{max}} = \tq \), we have \( \tp \leq t' \) and thus \( p - 1 \) divides \( k' \). This implies \( g^{k'} \equiv 1 \bmod p \). To obtain a nontrivial square root, we require \( g^{k'} \equiv -1 \bmod q \). Writing \( k' = \frac{q-1}{2}r' \) (with \( r' \) odd), by the lemma, if \( g \) is in the subgroup \( S \) of \( \mathbb{Z}_q^* \) (which has order \( \frac{q-1}{2} \)), then \( g^{k'} \equiv 1 \bmod q \); if \( g \notin S \), then \( g^{k'} \equiv (-1)^{r'} = -1 \bmod q \). Therefore, for a randomly sampled \( g \),

   $$
   \Pr[g^{k'} \equiv -1 \bmod q] = 1 - \Pr[g \in S] = 1 - \frac{\frac{q-1}{2}}{q-1} = \frac{1}{2}.
   $$

2. **Case 2: \( \tp = \tq \).**

   In this case, to obtain a nontrivial square root of 1, either
   - \( g^{k'} \equiv 1 \bmod p \) and \( g^{k'} \equiv -1 \bmod q \), or
   - \( g^{k'} \equiv -1 \bmod p \) and \( g^{k'} \equiv 1 \bmod q \).

   By a similar analysis as in Case 1, the total probability of a nontrivial square root occurring is

   $$
   \frac{1}{2} \cdot \frac{1}{2} + \frac{1}{2} \cdot \frac{1}{2} = \frac{1}{2}.
   $$

Thus, regardless of \( \tp \) and \( \tq \), we have:

$$
\Pr\left[\gcd\left(g^{k'} - 1, N\right) \text{ is a factor of } N\right] \geq \frac{1}{2}.
$$

Therefore,

$$
\Pr\left[\bigcup_{i=1}^{l} \gcd\left(g^{\frac{k}{2^i}} - 1, N\right) \text{ is a factor of } N\right] \geq \frac{1}{2}.
$$

This completes the proof.
{: .proof-end }

You can also programmatically verify the above proof using [this simulation](https://github.com/yuxqiu/garden/blob/main/2025-06-30-attacks-on-rsa/attacks-on-rsa.py). The simulation verifies the above proof and additionally provides another method to simulate the probability of factorizing \( N \) by checking all the elements in the sequence.

### Proof (Continued)

To factor \( N \), we can randomly sample group elements \( g \) until we find one that yields a nontrivial square root of 1. Based on the analysis above, this happens with a high probability after only a few tries. Ultimately, the nontrivial square root is used to factor \( N \)[^2].
{: .proof-end }

### Shor's Algorithm

The idea behind Shor's algorithm (period finding) is also connected to using square roots to factor \( N \).

Suppose we use Shor's algorithm to find an \( r \) such that

$$
f(x + r) = f(x),
$$

that is,

$$
(g^x)^r = g^x \quad \Rightarrow \quad g^{r-1} \equiv 1 \bmod N.
$$

Then, by the previous arguments, with probability at least \( \frac{1}{2} \), we can use \( g^{\frac{r-1}{2}},\ \dots,\ g^{\frac{r-1}{2^l}} \) to factor \( N \). Consequently, one can repeatedly sample elements and apply Shor's algorithm until success is achieved.

Another perspective on how a nontrivial square root aids in factoring \( N \) is to note that for an element with \( g^{r-1} \equiv 1 \bmod N \), we have:

$$
\left(g^{\frac{r-1}{2}} - 1\right) \left(g^{\frac{r-1}{2}} + 1\right) \equiv 0 \bmod N.
$$

Thus, one of the factors in the parenthesis will be a prime factor of \( N \) (provided \( g^{\frac{r-1}{2}} \neq \pm 1 \)).

## Next Steps

- Read [Twenty Years of Attacks on the RSA Cryptosystem](https://crypto.stanford.edu/~dabo/abstracts/RSAattack-survey.html).
- Considering it has been over 25 years since this survey, what are the new attacks against RSA?

[^1]: The sequence computation and gcd calculation can be done in polynomial time. See [Proof of Fact 1](https://crypto.stanford.edu/~dabo/pubs/papers/RSA-survey.pdf?page=3).
[^2]: This is not the only method to attack RSA when both \( e \) and \( d \) are known. [This post](https://mzhang.io/posts/2018-10-26-twenty-years-of-rsa-attacks/) details an alternative approach that estimates \( \frac{ed - 1}{\phi(N)} \) to compute \( \phi(N) \), which then allows one to deduce \( p+q = N - \phi(N) + 1 \) and factor \( N \).