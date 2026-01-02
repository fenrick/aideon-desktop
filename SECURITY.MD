# Security Policy

This repository follows a security-by-default posture:

- No renderer HTTP; renderer and host communicate via preload IPC only.
- Desktop mode opens no TCP ports. Worker runs over pipes/UDS only.
- Exports must default to PII redaction.

If you discover a vulnerability, please open a private security issue or contact the maintainers
directly.
