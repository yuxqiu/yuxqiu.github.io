---
layout: post
title: "Speculative Decoding for Transformers"
date: 2025-07-06
tags: ["AI"]

featured: false
katex: true
---

> **tl;dr**: I summarized the speculative decoding method for transformers and elaborated on why it is correct.
{: .block-tip }

## Speculative Decoding

I recently came across the paper, [Fast Inference from Transformers via Speculative Decoding](https://arxiv.org/abs/2211.17192v2). It borrows the idea from speculative execution in CPUs and proposes a new sampling algorithm, called **speculative decoding**, which enables parallel sampling from LLM outputs and thus speeds up inference.

The background for this technique is that traditionally, to sample N tokens from an LLM, we need to run the LLM serially N times, which is extremely inefficient. However, with the help of speculative decoding, the sampling speed can be greatly improved. Moreover, it is compatible with a wide range of existing techniques (such as top-p, top-k, beam search, and so on).

**Main Idea**
- Use a smaller model to approximate the output of a larger model. Specifically, a certain number of output tokens are sampled from the smaller model and then adjusted based on whether they match the target model's distribution.
- This is feasible when ample computational resources are available, as determining whether the output from the smaller model matches that of the larger model requires the larger model to be run in parallel on each prefix corresponding to each output.

## Correctness

The correctness argument is presented on page 3, section 3.2 "Calculating $$\alpha$$," and in Appendix A.1. The following is an excerpt from the correctness proof:

> We will now show that for any distributions \( p(x) \) and \( q(x) \), the tokens sampled via speculative sampling from \( p(x) \) and \( q(x) \) are distributed identically to those sampled from \( p(x) \) alone. Let \( \beta \) be the acceptance probability (Definition 3.1).
>
> Note that since \( p'(x) = \text{norm}(\max(0, p(x) - q(x))) = \frac{p(x) - \min(q(x), p(x))}{\sum_{x'} (p(x') - \min(q(x'), p(x')))} = \frac{p(x) - \min(q(x), p(x))}{1 - \beta} \), the normalizing constant for the adjusted distribution \( p'(x) \) is \( 1 - \beta \), where the last equation follows directly from Lemma 3.3 and Theorem 3.5.
>
> Now:
>
> \[ P(x = x') = P(\text{guess accepted}, x = x') + P(\text{guess rejected}, x = x') \]
>
> Where:
>
> \[ P(\text{guess accepted}, x = x') = q(x') \min\left(1, \frac{p(x')}{q(x')}\right) = \min(q(x'), p(x')) \]
>
> And:
>
> \[ P(\text{guess rejected}, x = x') = (1 - \beta) p'(x') = p(x') - \min(q(x'), p(x')) \]
>
> Overall:
>
> \[ P(x = x') = \min(q(x'), p(x')) + p(x') - \min(p(x'), q(x')) = p(x') \]
>
> As desired. □

In particular, \( P(\text{guess rejected}, x = x') \) initially puzzled me. By definition, \( \beta \) is the expectation (over the distribution \( q(x) \)) of the rejection probability. So how does it relate to \( P(\text{guess rejected}, x = x') \)?

\[
\beta = E_{x \sim q(x)} \begin{cases}
1 & q(x) \leq p(x) \\
\frac{p(x)}{q(x)} & q(x) > p(x)
\end{cases} = E_{x \sim q(x)} \min\left(1, \frac{p(x)}{q(x)}\right) = \sum_x \min(p(x), q(x))
\]

It turns out we can expand this probability formula as follows:

\[
\begin{align*}
&\quad P(\text{guess rejected}, x = x')\\
&= P(\text{guess rejected}, x = x', q(x = x_1)) + \dots + P(\text{guess rejected}, x = x', q(x = x_n))\\
&= \sum_i P(\text{guess rejected}, x = x', q(x = x_i))
\end{align*}
\]

where

\[
P(\text{guess rejected}, x = x', q(x = x_i))
\]

denotes the probability that the guess was rejected when \( x_i \) was sampled from \( q(x) \). This expansion is necessary because when each \( x_i \sim q(x) \) is rejected, it triggers a resampling from \( p'(x) \), which can result in \( x = x' \) being selected.

Thus, we can complete the equation:

\[
\begin{align*}
&\quad \sum_i P(\text{guess rejected}, x = x', q(x = x_i))\\
&= \sum_i q(x_i) \left(1 - \min\left(1, \frac{p(x_i)}{q(x_i)}\right)\right) p'(x')\\
&= p'(x') \left(1 - \sum_i q(x_i) \min\left(1, \frac{p(x_i)}{q(x_i)}\right)\right)\\
&= (1 - \beta) p'(x')
\end{align*}
\]

where

\[
1 - \min\left(1, \frac{p(x_i)}{q(x_i)}\right)
\]

is the probability that \( x_i \sim q(x) \) was rejected.
{: .proof-end}

## Simulation

You can experiment with simulation and concrete probability calculations using the script [here](https://github.com/yuxqiu/garden/tree/main/2025-07-06-speculative-decoding) to verify that the algorithm is indeed correct.

## More

If you're interested in this topic, check out this [well-curated repository](https://github.com/hemingkx/SpeculativeDecodingPapers) surveying all speculative decoding strategies. It includes approaches not only for transformers but for many other models as well. I'm not an expert (not even a beginner) in this area, so that's all I can say for now.
