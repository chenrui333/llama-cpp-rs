# Versioned native runtime SDK

This is a minimal fork of upstream tag `0.1.154`. `llama-cpp-2`'s safe API is
unchanged. Linux maintainer SDK builds disable the optional subprocess feature
as described below. `llama-cpp-sys-2/build.rs` gains
automatic prebuilt SDK selection and an explicit offline external mode.
Git consumers do not fetch a llama.cpp submodule. Only a selected source build
resolves native source, pinned separately in `build_support/source-lock.json`.
No crates.io packages are published.

## Baseline and supported configurations

Baseline: `zvec-ai/zvec-grep`, branch `codex/cuda-static-linking`, commit
`cc8fa72938db42b648687fccd99b088e94520186`.
Native source: llama.cpp `5f55650a78f92aff4d48d671423e888fac0469ff`.
The embedded runtime locks select Linux SDKs from
`zvec-0.1.154-glibc-2.28-r1`. macOS and Windows retain their verified
`zvec-0.1.154-r1` and `zvec-0.1.154-cpu-r2` SDKs.
Branch `prebuilt/0.1.154-cpu-r2-glibc-2.28` is based on
`prebuilt/0.1.154-cpu-r2-glibc-2.31` at `b55399a`. It builds and tests Linux
SDKs on glibc 2.28 and pins the verified archives described below. The Linux
manifests and consumer locks record `LLAMA_SUBPROCESS=OFF`.
Branch `prebuilt/0.1.154-cpu-r2` isolates Windows ARM64 CPU support from the
CUDA r2 upgrade: the default runtime lock and CUDA architecture configuration
remain identical to `0e2d4b5`, while the CPU lock adds the verified ARM64 SDK.
Native ABI/configuration changes require a new bundle revision; bootstrap-only
changes can reuse the pinned native SDK.

| Rust target | Native linkage | sys features |
| --- | --- | --- |
| aarch64-apple-darwin | static, CPU/Metal | common, metal |
| x86_64-apple-darwin | static, upstream Apple backends | common |
| aarch64-unknown-linux-gnu | static, CPU | common |
| x86_64-unknown-linux-gnu | shared core + CPU variants + CUDA plugins | common, cuda, dynamic-link, dynamic-backends |
| x86_64-pc-windows-msvc | shared core + CPU variants + CUDA plugins | common, cuda, dynamic-link, dynamic-backends |
| x86_64-unknown-linux-gnu | static, CPU-only | common |
| x86_64-pc-windows-msvc | static, CPU-only | common |
| aarch64-pc-windows-msvc | static, CPU-only | common |

The CPU-only matrix includes Linux x64, Windows x64 and Windows ARM64 (eight
SDK configurations total). The embedded `build_support/cpu-runtime-lock.json`
pins Linux x64 from `zvec-0.1.154-glibc-2.28-r1` and both Windows targets from
`zvec-0.1.154-cpu-r2`. Windows ARM64 selects its
own static CPU SDK in both `auto` and `prebuilt`; enabling CUDA or overriding
the source compiler does not silently select a CPU or x64 archive. Existing
archives and CUDA loading/fallback logic are unchanged. Exact features and native
options select the archive before any download, using the existing verified cache.
The original CPU release passed real builds, external-consumer linkage and
CPU inference in
[run 39929930003](https://github.com/zvec-ai/llama-cpp-rs/actions/runs/39929930003).
The replacement Linux SDK's validation is recorded below; Windows retains
those original verified archives.
Matching `{common}` builds on these three targets use only the CPU archive in `auto` and `prebuilt`;
they never download a CUDA archive instead. The same run also verified the
Windows source-build C1083/header-path fix included in `5abf435`.

`common` is intentional: upstream's safe crate still enables the sys crate's
default features even when its own defaults are disabled, exactly as in zg.
Apple backend defaults, including embedded Metal shaders and Accelerate, are
left to the pinned upstream build. Intel macOS sets deployment target 14.0,
matching zg. OpenMP is disabled. Release native builds are used for both debug
and release Rust consumers. The new Linux GNU build recipe compiles SDKs and
their separately linked test consumers against pinned manylinux 2.28 images,
on both x64 and ARM64. ARM64 uses its native `ubuntu-24.04-arm` Actions runner;
the container supplies the compiler and sysroot. MSVC uses the dynamic CRT.

The Linux acceptance gate inspects version requirements of every shared library
(including both CUDA plugins) and the external-mode consumer. It rejects imports
newer than `GLIBC_2.28`, `GLIBCXX_3.4.25` or `CXXABI_1.3.11`, and retains
`linux-abi.json` plus `native-imports.txt` as evidence. The linked consumer also
covers static CPU SDKs, whose final symbol versions are established at link
time. Real CPU embedding runs in a clean AlmaLinux 8 runtime container with its
own system libraries and no CUDA toolkit or NVIDIA libraries. Both CUDA modes
run in glibc 2.28 runtime containers on the GPU runner. Each Linux execution
checks `getconf GNU_LIBC_VERSION`; a successful test on a newer host libc does
not satisfy this acceptance gate. Host NVIDIA driver requirements remain
separate from the SDK's glibc ceiling.

Link static Linux consumers in the supplied `cpu` builder (EL8 GCC Toolset 13)
when targeting this runtime baseline. Its linker supplies the nonshared C++
compatibility routines required by GCC 13 objects. The stock EL8 GCC 8 linker
alone cannot resolve all of those routines. This is a build requirement: the
resulting consumer is tested against the clean runtime's system libraries and
does not require GCC Toolset 13 to be installed on the deployment machine.
For ordinary `auto`/`prebuilt` consumption inside this image, clear its
maintainer-only compiler overrides while retaining the toolset on `PATH`:
`env -u CC -u CXX -u CUDAHOSTCXX LLAMA_CPP_BUILD_MODE=prebuilt cargo build ...`.
Otherwise the existing compiler-override guard selects a source build in
`auto` or rejects it in `prebuilt`. The CI's `runtime.py external` command
already uses explicit external mode.
Static archives do not set the final executable's libc/libstdc++ baseline by
themselves; a consumer linked on a newer distribution still needs its own ABI
audit.

`native/Dockerfile` has `cpu`, `cuda13` and `cuda12` build targets. They share
the glibc 2.28 build environment. Only CUDA toolkits are copied from the pinned
NVIDIA images; their system libraries and sysroots are not copied. The
`runtime-cpu`, `runtime-cuda12` and `runtime-cuda13` targets provide separate
glibc 2.28 acceptance environments. GPU runtime targets add the corresponding
CUDA runtime/cuBLAS libraries, without a compiler or a second libc. CUDA
architecture coverage and the single-core/fallback layout remain unchanged.

The Linux SDK recipe sets `LLAMA_SUBPROCESS=OFF` while retaining `common`.
The pinned upstream subprocess helper calls
`posix_spawn_file_actions_addchdir_np`, an interface unavailable on glibc 2.28.
These new Linux SDKs therefore do not provide the optional `common_subprocess`
helper. In upstream applications this switch also disables the multi-model
server router, launching external MCP servers through stdio, and all nonempty
built-in server tool lists through their shared subprocess gate, not only shell
tools. MTMD video import through external `ffmpeg`/`ffprobe` is automatically
disabled as well. This concerns launching an external MCP process, not zg's
own MCP server.

The current SDK does not build upstream servers, tools, apps or examples and
does not enable the `mtmd` feature. Its CPU/CUDA inference, embedding, reranking,
tokenizer, sampling and grammar functionality remain enabled. This is a native
configuration change and must be recorded in the new manifest and consumer
locks. Existing macOS/Windows SDK configurations and ordinary source-build
defaults are unaffected.

Use `linux_only=true, variant=default` to validate Linux ARM64 CPU and x64 CUDA
SDKs, and `linux_only=true, variant=cpu` for the separate Linux x64 static CPU
SDK. Both runs skip publication and can run concurrently. Promote their exact
archives only after build, ABI, CPU and GPU acceptance succeeds.

### Verified glibc 2.28 SDKs

The Linux SDKs were built at `88b42171648683e8b99ed5a72e0ee96936d35887` and
verified in [CPU run 42594247210](https://github.com/zvec-ai/llama-cpp-rs/actions/runs/42594247210)
and [ARM64/CUDA run 42594242740](https://github.com/zvec-ai/llama-cpp-rs/actions/runs/42594242740).
All three SDKs passed independent Rust-consumer linkage and real CPU embedding
on clean glibc 2.28. Both CUDA plugins offloaded 25/25 model layers on T4 in the
glibc 2.28 runtime containers; CPU/GPU cosine similarity exceeded 0.999 in both
modes. The CUDA SDK also passed CPU inference without NVIDIA libraries.
All audited shared libraries and consumers require at most `GLIBC_2.28`,
`GLIBCXX_3.4.25` and `CXXABI_1.3.11`. All 14 x64 CPU backend variants and both
CUDA architecture lists remain present.

The x64 CPU archive additionally passed a link/load check retaining every
`llama_*`, `ggml_*` and `gguf_*` C API export, including the Rust common wrappers,
against the clean glibc 2.28 runtime. This supplements the real embedding tests;
it does not imply that every API or CUDA architecture was exercised at runtime.
The updated locks also passed validation against all three real archives.
Ordinary `prebuilt` mode linked the x64 CPU consumer without rebuilding native
code and passed real embedding plus ABI checks on clean glibc 2.28.

Release `zvec-0.1.154-glibc-2.28-r1` promotes the exact CI SDK archives without
rebuilding or repackaging. Archive names and manifest release labels retain
their original ABI-family identifiers. Each Linux lock entry overrides the
download `release` and records the native `source_commit`, archive digest,
manifest digest and configuration. Targets without an override use the lock's
top-level release. Cache identity remains the archive SHA-256. The Release
includes both consumer locks, checksums, CI evidence, supplemental API evidence
and `provenance.json`.

### Previously published glibc 2.31 SDKs

The existing Linux SDKs were built at `f72a0f29a49e9888d45ab95171fd77338f1bfd99` and
verified in [CPU run 40442188943](https://github.com/zvec-ai/llama-cpp-rs/actions/runs/40442188943)
and [ARM64/CUDA run 40442194019](https://github.com/zvec-ai/llama-cpp-rs/actions/runs/40442194019).
All tested ELF requirements are at most `GLIBC_2.30`; the supported baseline
remains glibc 2.31. Both CUDA plugins also passed real T4 GPU offload tests.

CUDA 13.1.1 primary build architectures, unchanged from the pinned r1 SDK:
`75-real;80-real;86-real;89-real;90-real;90-virtual`.
The CUDA r2 architecture upgrade is deliberately not included in this branch.
CUDA 12.4.1 fallback architectures:
`50-virtual;61-virtual;70-virtual;75-virtual;80-virtual;86-real`.
The fallback is **only** `lib/cuda12/ggml-cuda.dll` or
`lib/cuda12/libggml-cuda.so`, never a second core library. NCCL is disabled.
NVIDIA driver/runtime libraries are supplied by the deployment environment.

## Consumer contract

Pin the fork Git commit in the consuming repository. Archive/manifest digests
and native configuration are embedded in the sys crate's
`build_support/runtime-lock.json` (plus `cpu-runtime-lock.json` for CPU-only SDKs);
consumers do not maintain a second lock file
or build wrapper. Do not track a moving branch or fetch a remote checksums file
as the source of trust on every build.

Ordinary `cargo build` and `cargo test` default to `LLAMA_CPP_BUILD_MODE=auto`:

- Matching target, resolved sys features and native configuration download once
  and reuse the SHA-256-pinned Release SDK, including generated bindings.
- Native options that differ from the SDK select upstream's normal source
  build and print the reason. Examples include a different CUDA architecture
  list, enabling Vulkan/OpenMP, `GGML_CPU_REPACK=OFF`, `CXXFLAGS`, a lower macOS
  deployment target, or `-C target-cpu=native`.
- Normal Rust application changes, Rust debug/release profiles, logging and
  `-C debuginfo=1` reuse the same Release native SDK. To request native Debug,
  set `LLAMA_LIB_PROFILE=Debug` explicitly.
- `LLAMA_CPP_BUILD_MODE=prebuilt` requires a match; `source` always compiles
  from source. Unsupported targets/features use source mode in `auto`.
- Download/integrity failures are errors, not silent source fallbacks.

Source builds fingerprint native options and reconfigure CMake when they
change. Successful configuration is recorded only after the native build
completes, so failed attempts cannot reuse a partial cache as a valid one.

The cache defaults to `$CARGO_HOME/llama-cpp-runtimes` (normally
`~/.cargo/llama-cpp-runtimes`). `LLAMA_CPP_RUNTIME_CACHE` overrides its root.
SDKs are locked during atomic population and reverified when selected.
`LLAMA_CPP_RUNTIME_ARCHIVE=/path/to/pinned.tar.gz` can populate a cold cache
without network access, with the same digest checks. Curl is needed only for a
cold network download, not for an already-cached SDK.

For offline builds set `CARGO_NET_OFFLINE=true` or `LLAMA_CPP_OFFLINE=1`; a cache
miss then fails unless a local archive is supplied. Cargo's `--offline` flag
alone is not exposed to dependency build scripts and cannot control their
downloads. Existing explicit `external` mode below remains strictly offline.

### On-demand native source

The fork has no `.gitmodules` entry or gitlink for llama.cpp, so Cargo's Git
dependency acquisition does not download native source before `build.rs` runs.
`prebuilt`, matching `auto`, and `external` never resolve source, even on a
cold Cargo Git cache. A failed SDK download/checksum remains an error and does
not trigger source acquisition.

Only `source` or a real native-option mismatch in `auto` reaches the source
resolver. It verifies the exact revision above using both a pinned GitHub
codeload archive SHA-256 and a deterministic full file-tree SHA-256. It never
checks out a branch, initializes a submodule, changes an existing Git checkout,
or updates global Git configuration.

Source-only controls (ignored when an SDK is selected):

- `LLAMA_CPP_SOURCE_DIR`: use an existing byte-identical pinned source tree,
  read-only, without needing curl or Git. Root `.git` metadata is ignored, but
  modified/extra source files, symlinks and converted line endings are rejected.
- `LLAMA_CPP_SOURCE_CACHE`: content-addressed cache root, defaulting to
  `$CARGO_HOME/llama-cpp-sources` (normally `~/.cargo/llama-cpp-sources`). Warm
  entries are reverified; concurrent cold fetches are locked and installed
  atomically. Corrupt entries or injected Git metadata are errors, not silently
  replaced. Directory membership and all pinned files participate in Cargo's
  freshness checks, including newly added files and vendor headers.
- `LLAMA_CPP_SOURCE_ARCHIVE`: populate a cold source cache from the exact pinned
  `.tar.gz`, with the same archive and extracted-tree checks, without network.

With no explicit source directory, a warm cache is preferred. An existing
legacy `llama-cpp-sys-2/llama.cpp` checkout is also reusable if clean and exactly
pinned; this ignored directory is preserved, not managed by Cargo. An explicit
source archive bypasses a legacy checkout, so it can be used without modifying
dirty local source. Otherwise the source archive is downloaded once, only when
a source build is needed.
Source archives are not bundled into the sys crate's package.

`LLAMA_CPP_OFFLINE=1` or `CARGO_NET_OFFLINE=true` prohibits both SDK and source
downloads. Existing verified caches, source directories or supplied archives
still work. A cold source miss reports the required revision and ways to seed
it offline. Cargo's `--offline` alone does not govern build-script networking.
Source builds still require the platform's bindgen/CMake/C++/CUDA toolchain.
`DEP_LLAMA_SOURCE_DIR` and `DEP_LLAMA_SOURCE_REVISION` identify the actual source
chosen by the sys build script. Changing its resolved location reconfigures
CMake rather than reusing a cache tied to the previous source path.

For fully manual external mode, fetch once (Python 3.12+ and curl), then build:

```sh
python native/fetch.py --target aarch64-apple-darwin \
  --sha256 <pinned-archive-sha256> --output /absolute/path/to/runtime
export LLAMA_CPP_BUILD_MODE=external
export LLAMA_CPP_RUNTIME_DIR=/absolute/path/to/runtime
export LLAMA_CPP_RUNTIME_MANIFEST_SHA256=<pinned-manifest-sha256>
cargo build --locked
```

The safe crate dependency must use `default-features = false`; enable `cuda`
and `dynamic-backends` together on the two CUDA targets. Unsupported feature
combinations, wrong target/version, source-build overrides, missing files,
extra files, symlinks and hash failures are rejected in explicit external mode.
That mode never downloads or falls back to source. Do not set the runtime
directory when selecting automatic or source mode.

Each archive contains per-target pre-generated `bindings.rs`, a `manifest.json`,
native link/runtime libraries under `lib/`, and licenses. Generated bindings
are produced alongside the matching binary by the maintainer, **not** by the
consumer. The sys crate still has its upstream Rust build dependencies; external
mode does not execute bindgen, CMake, a C++ compilation or nvcc. A normal Rust
linker, target SDK and platform C++ runtime remain necessary (including the Apple
compiler-runtime search path on macOS).

The manifest records the native feature set, emitted link libraries, relevant
CMake cache entries, compiler information, source revision and payload SHA256s.
`DEP_LLAMA_RUNTIME_DIR` / `DEP_LLAMA_RUNTIME_MANIFEST` are available to direct
sys dependents. The existing `DEP_LLAMA_BACKENDS_DIR` contract is preserved.

For distribution, copy required shared libraries/plugins to the application's
runtime layout, configure executable loader paths, and call the existing
`load_backends_from_path` with the relocated directory. The SDK includes linker
archives/import libraries as well as runtime files; it is **not** an end-user
application package. Avoid shipping the entire SDK when only runtime files are
needed. Linux plugins use `$ORIGIN`, with `$ORIGIN/..` for the fallback. Windows
consumers must arrange DLL search paths before process startup (or use zg's
existing delay-load setup). This step does not change zg's loader or packaging.

## Build and release

`native-release.yml` runs only via `workflow_dispatch`. Its full-matrix release
job remains restricted to `prebuilt/0.1.154`. This branch uses a separate Linux
promotion after acceptance; `linux_only=true` always skips automatic publication.
Ordinary pushes do not rebuild or republish native SDKs. The embedded locks
select the verified glibc 2.28 promotion above. Future native changes require a
new release name and verified hashes before consumer locks can be updated.
Its default variant retains the original five-platform and CUDA gates. Select
`variant=cpu` to build the three CPU-only SDKs on ordinary hosted runners,
without CUDA toolchain setup or GPU jobs, reusing the same packaging, external
consumer, real CPU inference and audit steps. This defaults to preparing metadata;
publication additionally requires `publish_cpu=true`. No existing release is
overwritten. Review and import the resulting `cpu-runtime-lock.json` into the sys
crate only after actual native acceptance; consumers never trust a live remote index.
Windows ARM64 uses the native `windows-11-vs2026-arm` runner, `clang-cl`/Ninja
and ARM64 MSVC SDK/CRT (the pinned ggml rejects `cl.exe` for ARM). It runs the
same external consumer and real CPU embedding checks, without CUDA.
Locally, `runtime.py primary --target <cpu-target> --variant cpu` builds one CPU
SDK, and `publish.py --assets <merged-assets> --variant cpu --prepare-only`
generates its pins without GitHub calls. These are maintainer tools, not ZG wrappers.
All five native builds and real CPU inference tests run on ordinary hosted
runners. **Only** the Linux x64 and Windows x64 runtime verification jobs use `gpu-runners`, labels
`gpu-linux-t4` and `gpu-windows-t4`. They download the already-built SDK and Rust
probe; they do not rebuild native code.

The primary build invokes the upstream Cargo build script, then harvests the
exact OUT_DIR reported by Cargo (no guessed paths or latest-artifact selection).
The fallback uses the same pinned CMake options/toolchains as zg. CMake cache
assertions catch configuration drift. The release archive is then re-extracted
at another path and used to compile the probe in external mode with intentionally
invalid libclang/C++/CUDA tool paths. Rust contract tests cover rejection cases.

Every platform runs a real checksum-pinned EmbeddingGemma 300M Q8 embedding and
checks dimension/finiteness. Linux also runs without NVIDIA libraries installed
in the clean glibc 2.28 runtime image. Linux GPU tests exercise CUDA 12-only and
CUDA 13 environments on the same glibc baseline; Windows retains its existing
GPU runtime setup. The tests assert actual layer
offload and the loaded plugin path, and compare CPU/GPU embeddings (cosine
similarity >= 0.999). Native dependency audits and inference logs are retained
as Actions artifacts. A published pre-release means all gates passed; it does
not claim every CUDA architecture was executed (hardware verification is T4).

The default release job publishes all five SDKs, per-target manifests, `SHA256SUMS` and
`runtime-lock.json`. Existing releases/assets are never overwritten. Upstream
crate publishing and automated update workflows are guarded to upstream's repo
and disabled in this fork. `main`, imported tags and published SDKs are not
modified. This branch removes the submodule registration while retaining the
same native revision/ABI. zg integration is a separate acceptance-gated step.

Local entry points: `python native/runtime.py --help`, `python native/e2e.py
--help`; unit tests: `python -m unittest discover -s native -p 'test_*.py' -v`.

Cargo artifact-layout contracts are also included in the native smoke tests.
Cargo 1.100 places test executables under each package's build output and no
longer creates the legacy profile `deps` directory; staging only uses that
legacy destination when Cargo creates it.
They can run without compiling native libraries:
`rustc --test llama-cpp-sys-2/build_support/artifacts.rs -o /tmp/llama-artifact-tests && /tmp/llama-artifact-tests`.

An opt-in macOS ARM64 verification uses a disposable Git snapshot and a genuinely
empty Cargo Home (not a path dependency). It guards native Git/source downloads,
CMake and bindgen, tests SDK failures without source fallback, then performs an
on-demand source build, offline reuse, real CPU/Metal inference and cache-tamper
checks. It never commits the working branch, pushes, publishes or dispatches CI:

```sh
python native/verify_lazy_source.py \
  --sdk-archive /absolute/path/to/pinned-macos-sdk.tar.gz \
  --model /absolute/path/to/embeddinggemma-300M-Q8_0.gguf
```

Pass `--source-archive /absolute/path/to/pinned-source.tar.gz` to avoid the source
network download in this opt-in test. Logs/report stay under `target/lazy-source-*`.
Other platform/backend execution must be verified separately.
