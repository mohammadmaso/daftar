# ADR-0014: Plain HTTP allowed only for loopback Git remotes

* Status: accepted
* Date: 2026-09-23

§12 requires TLS everywhere except loopback OAuth. The connect form additionally accepts
`http://127.0.0.1|localhost|[::1]` remotes: they never leave the machine, and they make device testing
possible (`tools/git_http_server.py` + `adb reverse`). Any non-loopback `http://` URL is rejected.
