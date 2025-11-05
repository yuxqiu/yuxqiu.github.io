---
layout: default
title: Writings
permalink: /writings/
nav: true
nav_order: 2
---

<h1 class="post-title">Writings</h1>

<div class="post">

<small>
This space features my unpublished writings as well as presentations I’ve created about my work and that of others. I share them because they help me clarify and deepen my understanding of various concepts, and I hope they might be useful to others too. Compared to typical blog posts, these writings and presentations tend to be longer and more exploratory. Where possible, I’ll add reflections and continue revising based on feedback I receive. Hope you enjoy exploring them.
</small>

<hr>

<div class="post-list">
  <table class="table table-sm table-borderless">
    <!-- Disable pagination as it overwrites the introductory text -->
    <!-- https://github.com/sverrirs/jekyll-paginate-v2/pull/75 -->
    <!-- {% if page.pagination.enabled %}
      {% assign postlist = paginator.posts %}
    {% else %}
      {% assign postlist = site.posts %}
    {% endif %} -->

    {% assign postlist = site.writings | reverse %}

    {% for post in postlist %}

    {% assign year = post.date | date: "%Y" %}
    {% assign tags = post.tags | join: "" %}
    {% assign categories = post.categories | join: "" %}

    <tr>
        <th scope="row">{{ post.date | date: '%b %d, %Y' }}</th>
        <td>
            {% if post.redirect == blank %}
            <a class="post-link" href="{{ post.url | relative_url }}">{{ post.title }}</a>
            {% elsif post.redirect contains '://' %}
            <a class="post-link" href="{{ post.redirect }}" target="_blank">{{ post.title }}</a>
            {% else %}
            <a class="post-link" href="{{ post.redirect | relative_url }}">{{ post.title }}</a>
            {% endif %}
        </td>
        </tr>
    {% endfor %}
  </table>
</div>

{% if page.pagination.enabled %}
<br>
{% include pagination.liquid %}
{% endif %}

</div>