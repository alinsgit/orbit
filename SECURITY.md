# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.x     | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT open a public issue**
2. Email the details to the maintainer or use [GitHub Security Advisories](https://github.com/alinsgit/orbit/security/advisories/new)
3. Include steps to reproduce the vulnerability
4. Allow reasonable time for a fix before public disclosure

## Scope

Security concerns relevant to Orbit include:
- Credential storage (deploy passwords stored via OS keyring)
- SSH/SFTP connection handling
- Local service management and privilege escalation
- File system access and path traversal
- MCP server command execution

## Response

We aim to acknowledge security reports within 48 hours and provide a fix or mitigation plan within 7 days.
