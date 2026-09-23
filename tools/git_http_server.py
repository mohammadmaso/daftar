#!/usr/bin/env python3
"""Loopback-only Git smart-HTTP server for manual and device testing.

Serves bare repositories under ROOT via `git http-backend`. Binds to 127.0.0.1 only; use
`adb reverse tcp:PORT tcp:PORT` to reach it from an Android emulator as http://127.0.0.1:PORT/.

    tools/git_http_server.py ROOT [PORT]
"""
import http.server
import os
import subprocess
import sys


class Handler(http.server.BaseHTTPRequestHandler):
    def _cgi(self):
        path, _, query = self.path.partition("?")
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            body = b""
            while True:
                size = int(self.rfile.readline().strip().split(b";")[0], 16)
                if size == 0:
                    self.rfile.readline()
                    break
                body += self.rfile.read(size)
                self.rfile.readline()
        else:
            n = int(self.headers.get("Content-Length") or 0)
            body = self.rfile.read(n) if n else b""
        length = len(body)
        env = {
            **os.environ,
            "GIT_PROJECT_ROOT": ROOT,
            "GIT_HTTP_EXPORT_ALL": "1",
            "REQUEST_METHOD": self.command,
            "PATH_INFO": path,
            "QUERY_STRING": query,
            "CONTENT_TYPE": self.headers.get("Content-Type", ""),
            "CONTENT_LENGTH": str(length),
            "REMOTE_USER": "tester",
            "REMOTE_ADDR": "127.0.0.1",
        }
        if self.headers.get("Content-Encoding"):
            env["HTTP_CONTENT_ENCODING"] = self.headers["Content-Encoding"]
        out = subprocess.run(["git", "http-backend"], input=body, env=env, capture_output=True).stdout
        head, _, payload = out.partition(b"\r\n\r\n")
        status = 200
        headers = []
        for line in head.split(b"\r\n"):
            k, _, v = line.decode().partition(":")
            if k.lower() == "status":
                status = int(v.strip().split()[0])
            elif k:
                headers.append((k, v.strip()))
        self.send_response(status)
        for k, v in headers:
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    do_GET = _cgi
    do_POST = _cgi


if __name__ == "__main__":
    ROOT = os.path.abspath(sys.argv[1])
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 8765
    http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
