# Security policy

Use GitHub's [private vulnerability reporting](https://github.com/klik-in/skillbridge-oss/security/advisories/new) once the maintainer enables it for this repository. If the option is unavailable, ask for a private reporting channel from the maintainer without sharing vulnerability details publicly.

Include the affected commit, environment, minimum steps to reproduce, expected and observed access, and impact. Use synthetic accounts and redact credentials, session tokens, student data, screen captures and lab addresses. Please allow coordination before public disclosure.

This foundation is intended for local evaluation with synthetic data. Before a school pilot, review [deployment boundaries](docs/architecture.md#before-a-school-pilot) and the [lab checklist](docs/veyon.md#lab-checklist). The default Compose network publishes only the HTTP web port on loopback. Use HTTPS for any access beyond the host. Veyon remains disabled unless explicitly enabled and configured on the lab LAN.
