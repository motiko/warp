# Security Policy

## Reporting a Vulnerability

If you believe you have found a security vulnerability in Alefbet, please follow responsible
disclosure practices and **do not** open a public GitHub issue or pull request, as this could
expose the vulnerability before a fix is available.

Instead, report it privately through GitHub's private vulnerability reporting: open the
**Security** tab of this repository and choose **Report a vulnerability** to create a private
security advisory. Only the repository maintainers can see the report.

Please include steps to reproduce, the affected version or commit, and any relevant logs or
screenshots. We will acknowledge the report and work with you on a fix before anything is
published.

## Scope

Alefbet runs entirely on the local machine and does not talk to any server, so most reports will
concern local behavior: the terminal emulator, shell integration, file handling, or the build and
packaging scripts. Issues that only affect upstream Warp's hosted services are out of scope for
this repository.
