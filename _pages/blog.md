---
layout: default
permalink: /blog/
title: blog
nav: true
pagination:
  enabled: true
  collection: posts
  permalink: /page/:num/
  per_page: 20
  sort_field: date
  sort_reverse: true
  trail:
    before: 1 # The number of links before the current page
    after: 3 # The number of links after the current page
---

{% assign blog_name_size = site.blog_name | size %}
{% if blog_name_size > 0 %}
    <h1 class="post-title">{{ site.blog_name }}</h1>
{% endif %}


<div class="post">

{% assign featured_posts = site.posts | where: "featured", "true" %}
{% if featured_posts.size > 0 %}
<br>

<div class="container featured-posts">
{% assign is_even = featured_posts.size | modulo: 2 %}
<div class="row row-cols-{% if featured_posts.size <= 2 or is_even == 0 %}2{% else %}3{% endif %}">
{% for post in featured_posts %}
<div class="col mb-4">
<a href="{{ post.url | relative_url }}">
<div class="card hoverable">
<div class="row g-0">
<div class="col-md-12">
<div class="card-body">
<div class="float-right">
<i class="wh1">{% fa_svg fas.fa-thumbtack %}</i>
</div>
<h3 class="card-title text-lowercase">{{ post.title }}</h3>
                    {% assign year = post.date | date: "%Y" %}
                  </div>
                </div>
              </div>
            </div>
          </a>
        </div>
      {% endfor %}
      </div>
    </div>
    <hr>

{% endif %}

<div class="post-list">
  <table class="table table-sm table-borderless">
    {% if page.pagination.enabled %}
      {% assign postlist = paginator.posts %}
    {% else %}
      {% assign postlist = site.posts %}
    {% endif %}

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
{% include pagination.liquid %}
{% endif %}

</div>
