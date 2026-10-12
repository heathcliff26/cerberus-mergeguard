# Changelog

Version v0.5.6
--------------------
Released: 2026-10-12

🚀 Features:
- *(container)* Update image docker.io/library/rust to v1.99.0

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/alpine to v3.24.2

⚙️ Miscellaneous Tasks:
- Add automatic version update
- Check for conventional commit messages

💼 Other:
- Changes release workflow default settings
- Fix coverprofiles permissions

Version v0.5.5
--------------------
Released: 2026-09-12

🚀 Features:
- *(container)* Update image docker.io/library/rust to v1.98.1

💼 Other:
- Move helm to release workflow

Version v0.5.4
--------------------
Released: 2026-08-12

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/rust to v1.97.1
- *(deps)* Update rust crate jsonwebtoken to 11.0.0

Version v0.5.3
--------------------
Released: 2026-07-12

🚀 Features:
- *(container)* Update image docker.io/library/rust to v1.97.0

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/alpine to v3.24.1
- *(deps)* Update cargo

Version v0.5.2
--------------------
Released: 2026-06-12

🚀 Features:
- *(container)* Update image docker.io/library/alpine to v3.24.0

💼 Other:
- Move renovate and Link Check to tekton
- Remove obsolete Renovate status badge

Version v0.5.1
--------------------
Released: 2026-06-01

🚀 Features:
- *(container)* Update image docker.io/library/rust to v1.96.0

Version v0.5.0
--------------------
Released: 2026-05-15

💼 Other:
- Add envFrom section for injecting secrets
- Run workflow when changing the manifest repository
- Add default value for httpRoute rules
- Add namespace to backendRef
- Drop empty webhook config option

Version v0.4.2
--------------------
Released: 2026-04-29

💼 Other:
- Narrow focus of e2e test command
- Add tweaks to reduce binary size
- Fix permissions for coverprofile workflow

Version v0.4.1
--------------------
Released: 2026-04-18

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/alpine to v3.23.4
- *(deps)* Update cargo

Version v0.4.0
--------------------
Released: 2026-03-21

💼 Other:
- Use user and group nobody

Version v0.3.10
--------------------
Released: 2026-03-01

🐛 Bug Fixes:
- *(deps)* Update cargo

💼 Other:
- Remove audit workflow

Version v0.3.9
--------------------
Released: 2026-02-01

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/alpine to v3.23.3

💼 Other:
- Remove unneeded prepare step in renovate workflow

Version v0.3.8
--------------------
Released: 2026-01-01

🚀 Features:
- *(container)* Update image docker.io/library/alpine to v3.23.2

🐛 Bug Fixes:
- *(deps)* Update cargo

🧪 Testing:
- Make file name optional for temporary certificates

💼 Other:
- Add workflow to audit dependencies
- Add test for counting unfinished check runs

Version v0.3.7
--------------------
Released: 2025-12-01

💼 Other:
- Enable signing artifacts in release workflow

Version v0.3.6
--------------------
Released: 2025-11-01

🐛 Bug Fixes:
- *(container)* Update image docker.io/library/alpine to v3.22.2

Version v0.3.5
--------------------
Released: 2025-10-01

💼 Other:
- Ignore check-suite webhook events

Version v0.3.4
--------------------
Released: 2025-09-01

🐛 Bug Fixes:
- *(deps)* Update rust crate tokio to 1.47.1
- *(deps)* Update cargo

Version v0.3.3
--------------------
Released: 2025-08-01

🐛 Bug Fixes:
- *(deps)* Update rust crate reqwest to 0.12.22
- *(container)* Update image docker.io/library/alpine to v3.22.1
- *(deps)* Update cargo

Version v0.3.2
--------------------
Released: 2025-07-01

🐛 Bug Fixes:
- *(deps)* Update rust crate reqwest to 0.12.21

Version v0.3.1
--------------------
Released: 2025-06-22

🐛 Bug Fixes:
- *(deps)* Update cargo

Version v0.3.0
--------------------
Released: 2025-06-13

🐛 Bug Fixes:
- *(deps)* Update cargo

Version v0.2.0
--------------------
Released: 2025-06-05

🧪 Testing:
- Add more webhook-event test scenarios

💼 Other:
- Mark codeblocks for syntax highlighting
- Add e2e tests
- Ignore depency code
- Remove unused fields
- Add test and testutils module to ignore list

Version v0.1.0
--------------------
Released: 2025-06-01

⚙️ Miscellaneous Tasks:
- *(container)* Update image docker.io/library/alpine to 8a1f59f

💼 Other:
- Ensure all public fields in lib.rs are documented

Version v0.0.2
--------------------
Released: 2025-05-31

💼 Other:
- Add README.md to rustdocs
- Add more test runs
- Fix missing openssl
- Disable tracing for healthz endpoint
- Fix missing ssl certificates
- Borrow instead of take output
- Build binaries for amd64 and arm64
- Reduce image size by using alpine
- Add instruction for installation
- Update to alpine 3.22.0 and pin sha
- Add workflow for cover profiles
- Ignore test.rs files
- Add release workflow
- Use cargo-version and cargo-release workflows

Version v0.0.1
--------------------
Released: 2025-05-29

💼 Other:
- Edit testcover-report badge

