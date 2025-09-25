FROM ruby:slim

ENV DEBIAN_FRONTEND noninteractive

LABEL authors="Amir Pourmand,George Araújo" \
      description="Docker image for al-folio academic template" \
      maintainer="Amir Pourmand"

# install system dependencies
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        git \
        imagemagick \
        inotify-tools \
        locales \
        nodejs \
        procps \
        python3-pip \
        zlib1g-dev \
        # rugged, jekyll-og-image
        cmake \
        gcc \
        libssl-dev \
        libzstd-dev \
        libvips-dev \
        pkg-config

# clean up
RUN apt-get clean && \
    apt-get autoremove && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*  /tmp/*

# set the locale
RUN sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && \
    locale-gen

# set environment variables
ENV EXECJS_RUNTIME=Node \
    JEKYLL_ENV=production \
    LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8

# create a directory for the jekyll site
RUN mkdir /srv/jekyll

# build deps: copy the Gemfile, Gemfile.lock, and _local to the image
ADD Gemfile.lock /srv/jekyll
ADD Gemfile /srv/jekyll
ADD _local /srv/jekyll/_local

# set the working directory
WORKDIR /srv/jekyll

# install dependencies
RUN bundle install

EXPOSE 8080

COPY bin/entry_point.sh /tmp/entry_point.sh

CMD ["/tmp/entry_point.sh"]
