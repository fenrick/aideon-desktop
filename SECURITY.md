# Security Policy

This repository follows a security-by-default posture:

- Tauri invoke is the security boundary; commands are capability-gated.
- Desktop mode uses in-process engines; no inter-process RPC.
- No renderer HTTP.
- Production builds load only local assets; no remote CDNs.
- Exports must default to PII redaction.

If you discover a vulnerability, please open a private security issue or contact the maintainers
directly.
