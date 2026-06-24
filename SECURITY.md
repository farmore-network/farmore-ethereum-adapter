# Security Policy

Farmore is an open, permissionless protocol that secures real value. We welcome scrutiny.

## Reporting a vulnerability

**Do not open a public issue for security vulnerabilities.**

Report privately to **security@farmore.network** (fallback: farmore.reply@gmail.com), or
via GitHub's private vulnerability reporting ("Report a vulnerability" on the Security
tab). Include a description, affected component/version, reproduction steps, and impact.

We aim to acknowledge within 72 hours and to keep you informed through triage and fix.
Please give us reasonable time to remediate before public disclosure. We credit reporters
who wish to be named.

## Scope

This repository is part of the Farmore protocol stack. Even where a repository holds no
funds itself, logic, type, or supply-chain errors can have downstream value impact, so all
code here is in scope: correctness of protocol logic, soundness of public interfaces, and
supply-chain integrity (dependencies, build, release).

## Out of scope

Vulnerabilities in third-party dependencies already reported upstream; issues requiring a
compromised developer machine; theoretical issues without a practical exploit path.
