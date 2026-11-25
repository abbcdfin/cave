# Project Constraints

Each limit is defined where linked; this list is the index.

- **Linux only**, on distros that restrict unprivileged user namespaces too; no root,
  setuid helper or daemon. [Platform](../docs/product_specification.md#platform),
  [Isolation Engine](../docs/architecture_design.md#architectural-components).
- **Startup overhead < 100 ms.** [Key Requirements](../docs/product_specification.md#key-requirements).
- **Clumsy-agent threat model:** deliberate escapes and network egress are out of scope.
  [Scope](../docs/product_specification.md#scope).
- **`bwrap` is an external runtime dependency**, not embedded.
  [Spike Outcomes](../docs/architecture_design.md#spike-outcomes--decisions).
- **Clean-chroot package builds block user namespaces**, so the `PKGBUILD`'s `check()`
  runs unit tests only. [Packaging](../docs/architecture_design.md#packaging--distribution).
- **cave is primarily for the owner's own use**: the owner's daily agents and toolchains
  come first. [Spike Outcomes › Implementation Language](../docs/architecture_design.md#spike-outcomes--decisions).
