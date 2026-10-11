# Security Policy

## Supported Versions

Security fixes are applied to the latest released version of `ebook-rs`.

| Version | Supported |
| ------- | --------- |
| 0.16.x  | Yes       |
| < 0.16  | No        |

## Reporting a Vulnerability

Please report suspected vulnerabilities privately via GitHub's
[security advisory](https://github.com/SV-stark/ebook-rs/security/advisories/new) form rather
than opening a public issue.

Include the crate version, the affected entry point (CLI, HTTP reader server, MCP server, WASM,
Python, UniFFI, C FFI), a proof-of-concept input if you have one, and the impact you observed.

We aim to acknowledge reports within 72 hours.

---

## Threat model

`ebook-rs` parses **untrusted, attacker-supplied documents**. A malformed or hostile eBook must
never be able to crash the process, exhaust memory, read or write files outside an explicitly
granted directory, or execute script in the reader's origin.

The HTTP reader server and the MCP server bind to `127.0.0.1` only. They are nonetheless reachable
by any local process and, historically, by web pages via DNS rebinding. Treat them as privileged.

## Hardening in place

These are the controls the codebase relies on. Please preserve them when modifying the parsers or
the servers.

### Input bounds

* **Decompression budgets.** Every format extractor charges *actually decompressed* bytes against
  an explicit budget (`archive::DecompressionBudget`). Sizes declared in a ZIP central directory
  are attacker-controlled, so a limit enforced against a declared size can be bypassed — an
  under-declared total even flips the archive into lazy mode and skips the eager budget.
* **Recursion depth.** Element-tree walkers (notably the FB2 renderer) carry an explicit depth
  limit. `roxmltree` imposes no nesting limit, so unbounded recursion is a stack overflow, and a
  stack overflow is a `SIGSEGV`: it cannot be caught by `catch_unwind` and kills the process.
* **`overflow-checks = true` is set for the release profile.** This is deliberate — it turns
  integer overflow into a panic rather than silent wraparound. It also means every numeric
  parameter crossing a trust boundary must be clamped before use, because `a * 4` on an
  `as_u64()`-derived value is a remotely reachable panic. Use `RagChunkConfig::sanitized()` and
  saturating arithmetic.
* Result sets are capped **during** the scan (`SearchEngine::search_limited`), not after
  materializing everything and truncating.

### Output safety

* Section HTML is passed through `section::sanitize_html_scripts`, which strips `<script>`,
  event-handler attributes, and dangerous URI schemes. The scheme check runs against a *normalized*
  URI probe: HTML entities (including the named `&Tab;` / `&NewLine;`) are decoded and ASCII
  tab/LF/CR are stripped, because browsers do the same before resolving a scheme. A contiguous
  prefix check alone is bypassable with `java<TAB>script:`.
* `/resource/` serves raw archive bytes in the reader's own origin, so active content types
  (XHTML, SVG, JavaScript, XML) are served as `application/octet-stream` instead.
* The reader CSP does **not** grant `script-src 'unsafe-inline'`. Book content is untrusted; the
  sanitizer is the only thing standing between it and same-origin script execution.

### Server and MCP

* Bearer authentication must be configured for any deployment beyond a trusted machine
  (`--auth-token` / `EBOOK_RS_AUTH_TOKEN`). When a token is set it is enforced on **every**
  state-changing and book-content route, compared in constant time, and never accepted as an empty
  string.
* Filesystem-touching MCP tools are confined to a sandbox root (`--sandbox-root` /
  `EBOOK_RS_SANDBOX_ROOT`, defaulting to the current directory) and are **refused** when no root
  is configured. Path resolution canonicalizes symlinks before checking containment, so a link
  inside the sandbox cannot point outside it.
* Request bodies are capped and the declared `Content-Length` is checked before any read.
  Concurrent connections are capped.
* `Referrer-Policy: no-referrer` is set; the auth token is not accepted in the query string.

### Known residual risks

* **Slow-loris.** `tiny_http` 0.12 returns `&mut dyn Read` from `Request::as_reader()`, which is
  not `Send`, so the read cannot be moved to a helper thread, and the crate exposes no
  socket-level read timeout. A client that trickles a small body can hold a connection open until
  it finishes. Impact is bounded by the concurrent-connection cap. Fixing this properly requires a
  different HTTP stack or an upstream `tiny_http` timeout API.
* **Prompt injection.** Book metadata and body text reach LLM prompts through the MCP `prompts/*`
  methods. The fields are sanitized and explicitly labelled as untrusted data, but text-based
  prompt injection cannot be fully eliminated at this layer. Treat MCP prompt output as untrusted.