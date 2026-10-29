#!/usr/bin/env python3
"""Opt-in loopback integration checks. Starts HTTP/TLS services only from main().

Requires a built aquí binary, Python 3.11+, and openssl. No external requests,
package installation or credentials. Generates and deletes its own TLS fixture.
"""
import argparse
import base64
from contextlib import ExitStack, contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import ssl
import subprocess
import tempfile
import threading
import time
from urllib.parse import parse_qs, urlsplit


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_args):
        pass

    def handle(self):
        # Timeout/truncation/broken-pipe cases deliberately close keep-alive peers.
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError, TimeoutError):
            self.close_connection = True

    def read_body(self):
        self.connection.settimeout(2)
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            body = bytearray()
            while True:
                size = int(self.rfile.readline(128).split(b";", 1)[0], 16)
                if size == 0:
                    while self.rfile.readline(4096) not in (b"\r\n", b""):
                        pass
                    return bytes(body)
                if len(body) + size > 2 * 1024 * 1024:
                    raise ValueError("fixture input limit exceeded")
                block = self.rfile.read(size)
                if len(block) != size or self.rfile.read(2) != b"\r\n":
                    raise ValueError("incomplete chunk")
                body.extend(block)
        size = int(self.headers.get("Content-Length", "0"))
        if not 0 <= size <= 2 * 1024 * 1024:
            raise ValueError("fixture input limit exceeded")
        body = self.rfile.read(size)
        if len(body) != size:
            raise ValueError("incomplete body")
        return body

    def handle_request(self):
        try:
            body = self.read_body()
            url = urlsplit(self.path)
            query = parse_qs(url.query)
            if url.path in ("/redirect", "/loop"):
                self.send_response(int(query.get("code", ["302"])[0]))
                self.send_header("Location", "/loop" if url.path == "/loop" else query.get("to", ["/echo"])[0])
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            if url.path == "/slow-headers":
                time.sleep(0.5)
            if url.path == "/truncated":
                self.send_response(200)
                self.send_header("Content-Length", "20")
                self.end_headers()
                self.wfile.write(b"short")
                self.wfile.flush()
                self.close_connection = True
                return
            if url.path.startswith("/status/"):
                code = int(url.path.rsplit("/", 1)[1])
                payload = b"" if code in (204, 304) else b"\x00\xffbody"
            elif url.path == "/large-body":
                code, payload = 200, b"x" * (1024 * 1024)
            else:
                code = 200
                payload = json.dumps({"method": self.command,
                    "body": base64.b64encode(body).decode(),
                    "headers": {name.lower(): self.headers.get_all(name) for name in self.headers.keys()},
                }).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json" if url.path == "/echo" else "application/octet-stream")
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("X-Duplicate", "one")
            self.send_header("X-Duplicate", "two")
            self.end_headers()
            if self.command != "HEAD":
                if url.path == "/slow-body":
                    self.wfile.write(payload[:1])
                    self.wfile.flush()
                    time.sleep(0.5)
                    payload = payload[1:]
                self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError, TimeoutError):
            self.close_connection = True

    do_GET = do_HEAD = do_OPTIONS = do_DELETE = do_PATCH = do_POST = do_PUT = handle_request


@contextmanager
def server(context=None):
    instance = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    instance.daemon_threads = True
    if context:
        instance.socket = context.wrap_socket(instance.socket, server_side=True)
    thread = threading.Thread(target=instance.serve_forever, kwargs={"poll_interval": 0.05}, daemon=True)
    thread.start()
    try:
        yield f"{'https' if context else 'http'}://127.0.0.1:{instance.server_port}"
    finally:
        instance.shutdown()
        instance.server_close()
        thread.join(timeout=2)


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/aqui"))
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    env = {key: value for key, value in os.environ.items()
           if key.lower() not in {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}}
    env["NO_PROXY"] = "*"
    calls = 0

    def run(method, url, *options, input_data=None, expected=0):
        nonlocal calls
        calls += 1
        prefix = [] if "--color" in options else ["--color", "never"]
        result = subprocess.run([str(binary), *prefix, method, url, *options],
            input=input_data, capture_output=True, timeout=8, env=env)
        check(result.returncode == expected,
              f"case {calls} {method} {urlsplit(url).path}: exit {result.returncode}, expected {expected}; {result.stderr.decode(errors='replace')[:300]}")
        return result

    for options, expected_text in [
        (["--version"], b"aqui 8.1.1"),
        (["--help"], b"Commands:"),
        (["post", "--help"], b"--payload-from-stdin"),
    ]:
        calls += 1
        result = subprocess.run([str(binary), *options], capture_output=True, timeout=8, env=env)
        check(result.returncode == 0 and expected_text in result.stdout, "CLI help/version mismatch")

    with ExitStack() as stack:
        origin = stack.enter_context(server())
        other = stack.enter_context(server())
        for method in ("get", "head", "options", "delete", "patch", "post", "put"):
            response = run(method, origin + "/echo", "-D")
            if method == "head":
                check(response.stdout == b"", "HEAD returned body bytes")
            else:
                record = json.loads(response.stdout)
                check(record["method"] == method.upper(), "wrong outbound method")
                check(record["body"] == "", "unexpected outbound body")
            if method in ("delete", "patch", "post", "put"):
                for data in (b"", b' {"n":123456789012345678901234567890,"n":2} ', "héllo".encode()):
                    record = json.loads(run(method, origin + "/echo", data.decode(), "-D").stdout)
                    check(base64.b64decode(record["body"]) == data, "literal upload bytes changed")
                    check(record["headers"]["content-length"] == [str(len(data))], "wrong byte length")
                data = bytes(range(256)) * 256  # 64 KiB, bounded binary stdin upload
                record = json.loads(run(method, origin + "/echo", "-I", "-D", input_data=data).stdout)
                check(base64.b64decode(record["body"]) == data, "stdin bytes changed")

        for raw in (False, True):
            for fail in (False, True):
                for status in (200, 201, 204, 302, 400, 404, 500):
                    options = (["-D"] if raw else []) + (["--fail"] if fail else [])
                    response = run("get", origin + f"/status/{status}", *options,
                                   expected=int(fail and status >= 400))
                    if raw:
                        check(response.stdout == (b"" if status == 204 else b"\x00\xffbody"), "raw body changed")

        for status, method, expected_method in ((301, "post", "GET"), (302, "post", "GET"),
                                               (303, "put", "GET"), (307, "post", "POST"), (308, "put", "PUT")):
            record = json.loads(run(method, origin + f"/redirect?code={status}", "payload", "-D").stdout)
            check(record["method"] == expected_method, "redirect method incorrect")
            check(base64.b64decode(record["body"]) == (b"payload" if status >= 307 else b""), "redirect body incorrect")
        stopped = run("get", origin + "/redirect", "--max-redirects", "0")
        check(b"[302]" in stopped.stdout, "zero redirect budget followed a redirect")
        run("get", origin + "/loop", "--max-redirects", "2", expected=1)
        response = run("post", origin + "/redirect?code=307", "-I", input_data=b"stream")
        check(b"[307]" in response.stdout, "streaming body was unexpectedly replayed")

        record = json.loads(run("get", origin + "/echo", "-u", "user", "-p", "pass", "-D").stdout)
        check(record["headers"]["authorization"] == ["Basic dXNlcjpwYXNz"], "Basic auth not received")
        record = json.loads(run("get", origin + "/redirect?to=" + other + "/echo", "-u", "user", "-p", "pass", "-D").stdout)
        check("authorization" not in record["headers"], "Authorization leaked across origins")
        run("get", origin + "/echo", "-u", "user", "-h", "Authorization: Bearer test", expected=1)
        rendered = run("get", origin + "/status/200", "-u", "user", "-p", "pass")
        check(b"[redacted]" in rendered.stdout and b"dXNlcjpwYXNz" not in rendered.stdout, "auth display leaked")
        duplicate = run("get", origin + "/echo")
        check(duplicate.stdout.count(b"x-duplicate : ") == 2, "duplicate response header lost")
        for path in ("/slow-headers", "/slow-body"):
            run("get", origin + path, "--timeout", "100", expected=1)
        for raw in (False, True):
            result = run("get", origin + "/truncated", *(["-D"] if raw else []), expected=1)
            if not raw:
                check(result.stdout == b"", "truncated normal response partially displayed")
        run("get", origin + "/echo", "--max-body-bytes", "1", expected=1)
        run("get", origin + "/echo", "--timeout", "0", expected=2)
        calls += 1
        process = subprocess.Popen([str(binary), "get", origin + "/large-body", "-D"],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        process.stdout.close()
        process.stdout = None
        try:
            _, errors = process.communicate(timeout=8)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
            raise
        check(process.returncode == 1 and errors == b"", "broken pipe must fail quietly")

        temporary = stack.enter_context(tempfile.TemporaryDirectory(prefix="aqui-tls-"))
        cert, key = Path(temporary) / "cert.pem", Path(temporary) / "key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            "-keyout", str(key), "-out", str(cert), "-subj", "/CN=127.0.0.1",
            "-addext", "subjectAltName=IP:127.0.0.1"], check=True, capture_output=True, timeout=15)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)
        secure = stack.enter_context(server(context))
        run("get", secure + "/echo", expected=1)
        record = json.loads(run("get", secure + "/echo", "-C", "-D").stdout)
        check(record["method"] == "GET", "TLS opt-out request failed")
        run("get", secure + "/echo", expected=1)
        automatic = run("get", origin + "/echo", "--color", "auto")
        check(b"\x1b[" not in automatic.stdout, "auto color added ANSI to piped output")
        colored = run("get", origin + "/echo", "--color", "always")
        check(b"\x1b[38;2;" in colored.stdout, "forced color missing")
        raw = run("get", origin + "/status/200", "-D", "--color", "always")
        check(raw.stdout == b"\x00\xffbody", "forced color changed raw bytes")
    print(f"PASS: {calls} bounded loopback HTTP/TLS subprocess cases")
    print("Not covered: trusted public TLS, actual terminal auto-color, blocked stdin cancellation, proxy behavior, memory/RSS, interrupted/failed stdin sources, redirected output failures other than broken pipes.")


if __name__ == "__main__":
    main()
