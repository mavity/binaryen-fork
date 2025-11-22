# Special build notes

This note documents only what differs from the usual build; it assumes the reader
already knows how to run CMake and their normal generator (Ninja or Make).

- Resource constraints (CI/machine): limit parallelism and per-process memory
  - Use a low parallelism value to avoid exhaustion (example: set jobs to 1 or 2).
  - Example quick rule: JOBS=$(( $(nproc) > 2 ? 2 : $(nproc) )) then build with
    - `cmake --build build -j $JOBS` or `ninja -j $JOBS`.
  - Optionally pin compiles to a subset of CPUs to keep background load low
    (example: `taskset -c 0-1 ...`).
  - When memory pressure is the issue, set an address-space limit per-process
    with `ulimit -v <KB>` (e.g., ~2GB = `ulimit -v 2000000`).

- Build subset to reduce memory/CPU footprint
  - When constrained, prefer building only specific targets (e.g., `--target wasm-opt`) instead of `ALL_BUILD`.
  - This avoids heavy link steps and long compile chains in CI.

- Avoid heavy dependencies for constrained runs
  - Disable tests while building under strict constraints: pass `-DBUILD_TESTS=OFF` to CMake. This avoids pulling in `googletest` and reduces build size/time.

- CI ergonomics
  - For CI, prefer setting `CMAKE_BUILD_PARALLEL_LEVEL` or `-- -j <n>` rather than very low system-wide limits.
  - Use container/cgroup-level memory limits for isolation if available (less error-prone than low `ulimit`).

- What this note does not cover
  - Standard build instructions, general flags, or platform-specific toolchain steps (these are in `README.md`).
  - Emscripten or browser-specific build rules are unchanged and not covered here.

