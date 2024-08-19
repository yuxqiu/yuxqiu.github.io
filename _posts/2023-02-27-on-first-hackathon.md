---
layout: post
title: "On First Hackathon"
date: 2023-02-27 14:47:52
tags: ["Python"]

toc: true
mathjax: false
featured: false
published: false
---

This post aims to record something I learned from my first Hackathon experience.

## Python

I am responsible for setting up a set of REST APIs in the backend and building a dashboard for managing and displaying patient data in real-time.

The Tech Stack I chose for the REST APIs is Flask + SQLITE.

### Flask

Flask has good support for building the RESTful API. The only thing you need to do is to install [flask-restful](https://flask-restful.readthedocs.io/en/latest/).

It enables you to implement the logic of the API in a class and provides built-in support for the argument parsing that can parse data sent via `application/json`.

```python
# an example from their website
from flask import Flask
from flask_restful import Resource, Api

app = Flask(__name__)
api = Api(app)

class HelloWorld(Resource):
    def get(self):
        return {'hello': 'world'}

api.add_resource(HelloWorld, '/')

if __name__ == '__main__':
    app.run(debug=True)
```

It can automatically return error messages if the API requests from the clients are incorrect (missing parameters, type mismatch, or useless parameters).

To use the database, I wrap the `sqlite3` library. See [the code here](https://github.com/2023-Hackathon/patient-management-portal/blob/main/backend/database.py).

As I want to reuse the same database connection in every request, I save this to Flask's [Application Context](https://flask.palletsprojects.com/en/2.2.x/appcontext/) by following the [instructions](https://flask.palletsprojects.com/en/2.2.x/patterns/sqlite3/) at the website.

### Database

This part documents some simple commands I learned at the Hackathon.

The following are some commands that we use:

```sql
CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER, gender TEXT)
CREATE TABLE IF NOT EXISTS anxieties (id INTEGER PRIMARY KEY, user_id INTEGER, timestamp INTEGER, value INTEGER UNIQUE(user_id, timestamp) ON CONFLICT REPLACE, FOREIGN KEY (user_id) REFERENCES users(id))

INSERT or REPLACE into anxieties (user_id, timestamp, value) VALUES (?, ?, ?)
```

- Unique based on two (or more) columns `UNIQUE(col1, col2)`.
- `INSERT OR REPLACE` inserts if the value doesn't exist and replaces if it already exists. It's worth noting that `REPLACE` is an [alias](https://www.sqlite.org/lang_replace.html) of `INSERT OR REPLACE` in SQLite.


## Frontend

For convenience, we chose `streamlit` to visualize our data in the database.

It's a very easy-to-use library for displaying data. You only need to write some markdown, load the data into the appropriate format (`pd.DataFrame` or using `matplotlib` or ...), and `write` them to the rendered webpage.

Here is an example from their [website](https://streamlit.io/):

```python
import streamlit as st
import pandas as pd

st.write("""
# My first app
Hello *world!*
""")

df = pd.read_csv("my_data.csv")
st.line_chart(df)
```

They provide a variety of components to visualize your data. We utilized the `line_chart` and `bar_chart` in our project.

One thing that troubles us in the dev process is how to display our data in real-time as we don't find anything in [the official documentation for the `line_chart`](https://docs.streamlit.io/library/api-reference/charts/st.line_chart).

We finally strived to find the solution in [a discussion](https://discuss.streamlit.io/t/how-to-animate-a-line-chart/164) (though it turns out that they published a solution in [their blog](https://blog.streamlit.io/how-to-build-a-real-time-live-dashboard-with-streamlit/)).
By mimicking the usage of the `pyplot`, we find that the `st.line_chart` method returns a `delta` object. By updating the `delta` object, we can achieve real-time refreshing. This is how we do: [a loop](https://github.com/2023-Hackathon/patient-management-portal/blob/e356e5faf27e2ae19efb2da63ce088515e3d254a/frontend-user/user.py#L32) and [a method](https://github.com/2023-Hackathon/patient-management-portal/blob/e356e5faf27e2ae19efb2da63ce088515e3d254a/frontend-user/user.py#L92).


## Workflow

Some tips when working with database and `requests` library next time:
- Encapsulate database actions in functions. This saves you from calling `db.commit()` and hides the implementation details from the call site.
- Encapsulate the `requests` library when doing network requests. This can save you a bunch of error-handling codes on the call site.


## Team Management

1. Break the project into separate parts
2. For each part, talk to every member of your team to specify their jobs
3. For each job, specify how much time they want
4. Setup a deadline for every part
5. Keep Working!
6. Synchronization: dynamically adjust based on the progress
   1. Ideally after developing every part
   2. Can be a short standup meeting
7. Go back to 2