#!/usr/bin/env python3
print("Content-Type: text/html")
print()
print("<h1>CGI Test</h1>")
print("<p>PATH_INFO: {}</p>".format(__import__("os").environ.get("PATH_INFO", "")))

