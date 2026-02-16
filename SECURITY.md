# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in stacy, please report it by emailing
jfasnacht@uchicago.edu rather than opening a public issue.

Please include:
- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact

You can expect an initial response within 48 hours.

## Scope

stacy is a build-time tool that wraps Stata execution. Security concerns include:
- Command injection through script paths or arguments
- Improper handling of user-provided configuration
- Package installation from untrusted sources

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
