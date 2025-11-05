---
layout: post
title: "Trustless Efficient Light Clients"
date: 2025-05-01
tags: ["Cryptography"]

featured: false
jektex: true
---

<div class="links">
    <a href="/assets/pdf/writings/2025/trustless-efficient-light-client.pdf" class="btn btn-sm z-depth-0" role="button" rel="external nofollow noopener" target="_blank">PDF</a>

    <a href="https://github.com/yuxqiu/mim" class="btn btn-sm z-depth-0" role="button" rel="external nofollow noopener" target="_blank">Code</a>
</div>

## TL;DR

We leveraged folding-based SNARKs to design an efficient light-client protocol and introduced a novel data structure, the Levelled Merkle Forest (LMF), which enables variable-length Merkle proofs. We also addressed a critical bug in `EmulatedFpVar` in `arkworks`. Evaluation shows that LMF performs comparably to standard Merkle trees while producing smaller proof sizes, though the practicality of the scheme is currently constrained by the large memory requirements for proving pairings.

## Abstract

Light clients play a crucial role in blockchain ecosystems by enabling devices with limited computational resources to participate in on-chain activities. However, achieving full security typically requires downloading data that scales linearly with the blockchain's size. This poses a major scalability challenge as blockchains grow, ultimately undermining the very utility of light clients.

In this work, we propose [Mim](https://github.com/yuxqiu/mim), a system that leverages *succinct non-interactive arguments of knowledge* (SNARKs) to efficiently prove the correctness of committee rotations in Byzantine Fault Tolerant (BFT)-based light client protocol. Unlike prior approaches that either compress downloadable data or redesign consensus mechanisms around zero-knowledge proofs, our solution is generalizable and achieves verification performance that is independent of or sub-linear in the chain size.

Our core contribution is the use of *folding-based SNARKs*, a recent advancement that enables multiple instances of identical NP statements - in our case, committee rotation verifications - to be folded into a single proof. When paired with an appropriate proving system, this allows us to generate constant-size $$O(1)$$ proofs of committee rotation. To further enhance efficiency, we introduce a novel data structure called the **Levelled Merkle Forest (LMF)**, which mitigates the need to run SNARKs for every committee rotation. LMF accumulates committee data into hierarchical Merkle trees, enabling variable-length proofs post-verification.

We implement our protocol, including the newly introduced LMF data structure, in Rust with over 8000 lines of code. During this process, we implemented the first open-source, generic R1CS implementation of the hash-to-curve algorithm in `arkworks`, and fixed a critical bug in the emulated field variable that generated unsatisfiable constraints.

We evaluate our design on a simulated blockchain modelled after the checkpoint structure of Sui. Our extrapolated results show that, given sufficient memory, it is feasible to generate a committee rotation proof in under half a day (without LMF), faster than the typical one-day checkpoint generation interval. Moreover, with LMF, proof covering one year of checkpoint data can be produced in under a week. Concurrently, LMF achieves performance comparable to the Merkle tree while reducing proof sizes for all of the committed values, making it a practical and efficient alternative. We conclude by highlighting several promising directions for future work to further enhance the efficiency and applicability of our system.

## Citation

```
@misc{qiutrustless2025,
  author = {Yuxiang Qiu},
  title = {Trustless Efficient Light Clients},
  year = {2025},
  url = {https://yuxqiu.github.io/assets/pdf/writtings/2025/trustless-efficient-light-client.pdf},
  note = {Preprint}
}
```