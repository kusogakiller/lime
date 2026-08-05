# Requests Standard Library (Phase C-1.13)

HTTP client library for Lime, providing synchronous HTTP client functionality compatible with Python requests.

## Overview

The requests module provides a high-level HTTP client API for making HTTP requests in Lime programs. It supports:

- HTTP methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- Request headers and cookies
- Request bodies: string, bytes, JSON, form data, multipart
- Response handling: status codes, headers, body text/bytes/JSON
- Authentication: Basic, Bearer token
- TLS/SSL configuration with certificate validation
- Redirect following with history tracking
- Timeouts
- Sessions for persistent connections and cookie management

## Architecture

The requests module uses opaque handles managed by the runtime:
- **Interpreter mode**: Uses curl (POSIX) or shell commands directly
- **Native codegen mode**: Uses C runtime implementations (WinHTTP on Windows, curl on POSIX)

## API Reference

### Top-Level Functions

```lime
fn request(str: method, str: url): RequestBuilder
```
Create a request builder for the specified HTTP method and URL.

```lime
fn get(str: url): Result(Response, RequestError)
fn post(str: url, str: body): Result(Response, RequestError)
fn put(str: url, str: body): Result(Response, RequestError)
fn patch(str: url, str: body): Result(Response, RequestError)
fn delete(str: url): Result(Response, RequestError)
fn head(str: url): Result(Response, RequestError)
fn options(str: url): Result(Response, RequestError)
```
Convenience functions for common HTTP methods.

```lime
fn session(): Session
```
Create a new session for persistent connections and cookies.

### Session

```lime
fn Session.get(Session: self, str: url): RequestBuilder
fn Session.post(Session: self, str: url): RequestBuilder
fn Session.put(Session: self, str: url): RequestBuilder
fn Session.patch(Session: self, str: url): RequestBuilder
fn Session.delete(Session: self, str: url): RequestBuilder
fn Session.head(Session: self, str: url): RequestBuilder
fn Session.options(Session: self, str: url): RequestBuilder
```
Create a request builder using the session's configuration.

```lime
fn Session.headers(Session: self, list(tuple(str,str)): hdrs)
fn Session.params(Session: self, list(tuple(str,str)): params)
fn Session.timeout(Session: self, int: seconds)
fn Session.verify(Session: self, bool: verify)
fn Session.cookies(Session: self): list(tuple(str,str))
```
Configure session defaults (Python requests compatible).

### RequestBuilder (Fluent API)

```lime
fn RequestBuilder.params(RequestBuilder: self, list(tuple(str,str)): params): RequestBuilder
fn RequestBuilder.data(RequestBuilder: self, str: body): RequestBuilder
fn RequestBuilder.json(RequestBuilder: self, opaque: json_val): RequestBuilder
fn RequestBuilder.headers(RequestBuilder: self, list(tuple(str,str)): hdrs): RequestBuilder
fn RequestBuilder.header(RequestBuilder: self, str: key, str: value): RequestBuilder
fn RequestBuilder.auth(RequestBuilder: self, str: user, str: password): RequestBuilder
fn RequestBuilder.verify(RequestBuilder: self, bool: verify): RequestBuilder
fn RequestBuilder.allow_redirects(RequestBuilder: self, bool: allow): RequestBuilder
fn RequestBuilder.timeout(RequestBuilder: self, int: seconds): RequestBuilder
fn RequestBuilder.send(RequestBuilder: self): Result(Response, RequestError)
```

### Response

```lime
fn Response.status(Response: self): int
fn Response.headers(Response: self): list(tuple(str,str))
fn Response.url(Response: self): str
fn Response.text(Response: self): str
fn Response.json(Response: self): opaque
fn Response.ok(Response: self): bool
fn Response.is_success(Response: self): bool
fn Response.is_client_error(Response: self): bool
fn Response.is_server_error(Response: self): bool
fn Response.error_for_status(Response: self): Result(Response, RequestError)
fn Response.content_length(Response: self): int
fn Response.copy_to(Response: self, str: file_path): int
fn Response.history(Response: self): list(str)
```

## Examples

### Basic GET Request

```lime
import requests

fn main():
    let resp = requests.get("http://httpbin.org/get")
    if resp.is_success():
        println(resp.text())
    return
```

### POST with JSON Body

```lime
import requests

fn main():
    let resp = requests.post("http://httpbin.org/post", "{\"key\": \"value\"}")
    if resp.is_success():
        println(resp.status())
    return
```

### Using RequestBuilder

```lime
import requests

fn main():
    let resp = requests.request("GET", "http://httpbin.org/get")
        .header("Authorization", "Bearer token123")
        .params([("key", "value")])
        .timeout(30)
        .send()
    if resp.is_success():
        println(resp.text())
    return
```

### Using Sessions

```lime
import requests

fn main():
    let s = requests.session()
    s.headers([("User-Agent", "Lime/1.0")])
    s.timeout(60)
    s.verify(true)
    let resp = s.get("http://httpbin.org/get").send()
    if resp.is_success():
        println(resp.text())
    let cookies = s.cookies()
    requests.session_free(s)
    return
```

### Redirect History

```lime
import requests

fn main():
    let resp = requests.get("http://example.com/redirect")
        .allow_redirects(true)
        .send()
    let history = resp.history()
    // history contains URLs that were redirected through
    return
```

## Security Notes

- TLS certificate verification is enabled by default (`verify=true`)
- Use `verify(false)` only for testing, never in production
- `verify(false)` disables certificate validation and hostname checking
- Cookies are properly parsed from Set-Cookie headers with domain/path matching
- Secure cookies are only sent over HTTPS
