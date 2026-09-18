# Environment & Tooling

## Toolchains
- **Rust**: Primary language for the `cave` CLI binary (`cargo`, `rustc` 1.94+).
- **Python**: Default toolchain is `uv` (`uv run ...`, `uv pip ...`) when Python scripts or tools are utilized.

## Sandbox Dependency
- **Bubblewrap**: `/usr/bin/bwrap` required on the host system.

## Script Placement
- Exploratory, supportive, or throwaway scripts belong in `./vdesign/utils/`, never in the production tree.

## Framework Integration
- Vibe framework conventions: volatile planning in `./vdesign/`, persistent artifacts in `./docs/`.
