# Default mirrors PYTHON_BASE_IMAGE in versions.env (the pin authority);
# build-images.sh passes it explicitly.
ARG PYTHON_BASE_IMAGE=python:3.13.14-slim@sha256:6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91
FROM ${PYTHON_BASE_IMAGE}

ARG CIRQ_VERSION
ARG QSIMCIRQ_VERSION

# qsimcirq publishes no linux/aarch64 wheel at any Python version, so this
# always builds from source; install the toolchain then strip it. git is
# required because qsim's CMake fetches pybind11 via FetchContent GIT_REPOSITORY.
#
# The two seds patch upstream assumptions that break every linux/aarch64 build
# (qsim through v0.22.0 and master, 2026-07-31). Both treat "not Apple Silicon"
# as "x86", so linux/aarch64 takes the x86 path:
#
#   1. decide.cpp guards its x86 CPUID feature detection with
#        #if !defined(__aarch64__) || !defined(__APPLE__)
#      which is true on linux/aarch64 (__APPLE__ is undefined), so it compiles
#      the x86 path and fails with "'cpuid' was not declared in this scope" --
#      cpuid only exists under _WIN32 or __x86_64__/__i386__. The intent was
#      clearly !(defined(__aarch64__) && defined(__APPLE__)). Restricting the
#      guard to the architectures that actually have cpuid makes aarch64 return
#      instr = BASIC, the same unvectorized path Apple Silicon takes.
#   2. CMakeLists.txt adds the sse/avx2/avx512 modules under
#      `if(NOT CMAKE_APPLE_SILICON_PROCESSOR)`, so aarch64 tries to compile them
#      and fails on "unrecognized command-line option '-msse4.1'". Dropping
#      those three subdirs leaves basic + decide -- exactly the module set
#      Apple Silicon builds, and the only one detect_instructions() can select
#      here. (CMAKE_APPLE_SILICON_PROCESSOR can't be overridden via CMAKE_ARGS:
#      CMakeLists.txt set()s it unconditionally, shadowing any -D cache entry.)
#
# The greps assert the unpatched text is present, so this fails loudly rather
# than silently no-op'ing once upstream fixes it and the patch should be dropped.
# libgomp1 is named explicitly so apt marks it manually-installed: qsim links
# against libgomp.so.1 at runtime, and as a mere g++ dependency it would be
# taken out by the autoremove below, leaving an image that builds but fails on
# `import qsimcirq` with "libgomp.so.1: cannot open shared object file".
RUN apt-get update && apt-get install -y --no-install-recommends cmake g++ git make libgomp1 \
    && pip install --no-cache-dir "cirq-core==${CIRQ_VERSION}" "ply" \
    && pip download --no-binary=qsimcirq --no-deps --no-cache-dir \
       "qsimcirq==${QSIMCIRQ_VERSION}" -d /tmp/qsim \
    && tar -xzf /tmp/qsim/qsimcirq-*.tar.gz -C /tmp/qsim \
    && SRC=/tmp/qsim/qsimcirq-${QSIMCIRQ_VERSION} \
    && grep -q '#if !defined(__aarch64__) || !defined(__APPLE__)' \
       "$SRC/pybind_interface/decide/decide.cpp" \
    && sed -i 's/#if !defined(__aarch64__) .. !defined(__APPLE__)/#if defined(__x86_64__) || defined(__i386__) || defined(_WIN32)/' \
       "$SRC/pybind_interface/decide/decide.cpp" \
    && grep -q 'add_subdirectory(pybind_interface/sse)' "$SRC/CMakeLists.txt" \
    && sed -i '/add_subdirectory(pybind_interface\/\(sse\|avx2\|avx512\))/d' \
       "$SRC/CMakeLists.txt" \
    && pip install --no-cache-dir "$SRC" \
    && rm -rf /tmp/qsim \
    && apt-get purge -y cmake g++ git make && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY run_circuit.py /app/run_circuit.py

ENTRYPOINT ["python", "/app/run_circuit.py"]
