# ADR 0002 — Development efficiency first: Docker on by default

- **Status:** Accepted, 2026-11-25. Built by increment 9 (**pending**).
- **Changes:** [product_specification.md › Scope](../product_specification.md#scope), which
  had hidden the Docker socket since 2026-09-17.

## Context

cave exists so coding agents can run without permission prompts. Its boundary keeps a
clumsy agent away from files outside the project, and until now it counted any service
that reaches host files as file access, so the Docker socket was hidden
(`docker run -v /:/host` reaches every host file as root). The owner's projects use
`docker compose`, so agents in cave could not build, run or test them, and the owner had
to leave the sandbox for that part of the work. The owner is cave's only user.

## Decision

**Development efficiency comes first.** The boundary covers the mistakes that are both
likely and costly to undo, chiefly reaching files outside the project, and gives way where
hiding something would slow daily development. Each such exception is named in the Scope
with its cost and can be turned off for a session.

**Docker is the first such exception, on for everyone.** A built-in `docker` toolchain
is on by default: cave mounts the socket the docker CLI would use (`DOCKER_HOST`, else
`/var/run/docker.sock`), probed at startup rather than fixed to one host's layout.
`--without docker` turns it off for a session, and the user file turns it off for good.
`~/.docker` stays hidden: its `config.json` holds registry credentials.

## Rejected alternatives

- **Keep the socket hidden:** agents cannot work on projects that run under Docker, which
  is daily work.
- **Opt-in with `--with docker`:** every session that needs Docker needs the flag, and a
  forgotten flag fails only once the agent reaches Docker.
- **Off in the built-in table, on in the owner's user file:** keeps the shipped default
  strict for other users, but there are none, and cave's defaults should serve its purpose.
- **A filtering proxy for the Docker API** (refusing bind mounts outside the allowed
  directories, or `prune`): a daemon cave would have to run, against the
  no-daemon constraint, and a large surface to get right.

## Consequences

- The Scope's promise that allowed directories are the only host files an agent can
  reach gains a named exception: through the daemon an agent can reach every host file,
  and remove other projects' containers, images and volumes (`docker system prune`).
- The daemon reads `-v` paths on the host. Project paths work, since cave keeps host
  paths; `-v ~/…` and `-v /tmp/…` reach the host's files, not the sandbox's.
- Releases list this change in `CHANGELOG.md`, as [ADR 0001](0001-releases.md) requires of
  any change to what the sandbox exposes.
