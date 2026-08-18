#!/usr/bin/env python3
"""
Phase 1 Iteration 8 — C ecosystem corpus validation gate.

Machine-checkable regression gate for the Charger C-only ABI work. No
library-specific charger logic: it enumerates corpus test cases from the tables
below, builds each slice with the freshly-built `lime` binary, executes it, and
compares stdout against the recorded expected output.

Two verification tiers:

  Tier 1 (strict, must pass): core C slices. Each has a deterministic expected
  stdout captured from a verified-correct run. Any mismatch -> non-zero exit.

  Tier 2 (store integrity): every installed C library in `.lime-charger/store`.
  Confirms the manifest parses, the native artifact (.lib/.a) exists, and every
  symbol referenced by the generated `lime-iface.lime` is present in the
  manifest's symbol list. This proves the prepared artifact is linkable without
  re-parsing the header. (Execution of real-world drivers is covered separately
  and may be blocked by known charger adapter-generation limitations.)

Usage:
  python3 bench_clang/validate_corpus.py
Exit code 0 = all strict checks passed; non-zero = regression detected.
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIME_EXE = os.path.join(ROOT, "target", "release", "lime.exe")
if not os.path.exists(LIME_EXE):
    LIME_EXE = os.path.join(ROOT, "target", "release", "lime")
SLICES_DIR = os.path.join(ROOT, "bench_clang", "charger", "slices")
STORE_DIR = os.path.join(ROOT, ".lime-charger", "store")

# Tier 1: core C slices with recorded expected stdout (whitespace-normalized).
CORE_SLICES = {
    "agg": "399271.52.5",
    "c_callback": "7",
    "c_dep": "6",
    "c_iter8": "34ITER8_OK",
    "c_math": "7749",
    "c_mathx": "7725",
    "c_ptr": "2",
    "cb": "1823042DESTROY_CALLED610",
    "gvar": "01025263hi42779997913100200144942",
    "layout": "64215",
    "semantic": "obj-424242535",
    "variant": "42102435",
    "variadic": "10600000000007177078603015",
}

LLVM_BIN = os.environ.get("LIME_LLVM_BIN", "")
ENV = dict(os.environ)
if LLVM_BIN:
    ENV["PATH"] = LLVM_BIN + os.pathsep + ENV.get("PATH", "")
    ENV["LIME_LLVM_BIN"] = LLVM_BIN


def norm(s: str) -> str:
    return re.sub(r"\s+", "", s).strip()


def build_and_run(slice_name: str):
    path = os.path.join(SLICES_DIR, slice_name + ".lime")
    exe = os.path.join(SLICES_DIR, slice_name + ".exe")
    if os.path.exists(exe):
        os.remove(exe)
    b = subprocess.run([LIME_EXE, "build", path], capture_output=True, text=True, env=ENV)
    if b.returncode != 0 or not os.path.exists(exe):
        return None, "build failed: " + (b.stderr or b.stdout)[-400:]
    r = subprocess.run([exe], capture_output=True, text=True, env=ENV)
    return r.returncode, r.stdout


def check_core():
    print("=== Tier 1: core C slices ===")
    fails = []
    for name, expected in CORE_SLICES.items():
        rc, out = build_and_run(name)
        if rc is None:
            print(f"  FAIL  {name}: {out}")
            fails.append(name)
            continue
        if rc != 0:
            print(f"  FAIL  {name}: run rc={rc} (segfault/crash)")
            fails.append(name)
            continue
        gn, en = norm(out), norm(expected)
        if gn != en:
            print(f"  FAIL  {name}: output mismatch got={gn!r} want={en!r}")
            fails.append(name)
            continue
        print(f"  PASS  {name}")
    return fails


def check_store_integrity():
    print("=== Tier 2: store integrity (installed C libraries) ===")
    fails = []
    if not os.path.isdir(STORE_DIR):
        print("  (no store)")
        return fails
    for lib in sorted(os.listdir(STORE_DIR)):
        libdir = os.path.join(STORE_DIR, lib)
        if not os.path.isdir(libdir):
            continue
        # newest version / hash entry
        entries = []
        for ver in os.listdir(libdir):
            vd = os.path.join(libdir, ver)
            if os.path.isdir(vd):
                for h in os.listdir(vd):
                    ed = os.path.join(vd, h)
                    if os.path.isdir(ed):
                        entries.append(ed)
        if not entries:
            print(f"  FAIL  {lib}: no store entry")
            fails.append(lib)
            continue
        ed = max(entries, key=os.path.getmtime)
        manifest = os.path.join(ed, "manifest.toml")
        iface = os.path.join(ed, "lime-iface.lime")
        artifact_name = None
        symbols = []
        if os.path.exists(manifest):
            txt = open(manifest, encoding="utf-8").read()
            m = re.search(r'artifact\s*=\s*"([^"]+)"', txt)
            artifact_name = m.group(1) if m else None
            # Extract symbols from the TOML `symbols = [...]` array, which may be
            # inline (`symbols = ["a", "b"]`) or multi-line. Pull every quoted
            # string inside the symbols array block.
            sm = re.search(r'symbols\s*=\s*\[(.*?)\]', txt, re.S)
            if sm:
                symbols = re.findall(r'"([^"]+)"', sm.group(1))
            else:
                symbols = []
        else:
            print(f"  FAIL  {lib}: manifest missing")
            fails.append(lib)
            continue
        if not artifact_name or not os.path.exists(os.path.join(ed, artifact_name)):
            print(f"  FAIL  {lib}: artifact {artifact_name} missing")
            fails.append(lib)
            continue
        # iface symbols must be a subset of manifest symbols, EXCEPT charger-
        # generated adapter shims (prefixed `lime_`) which are synthesized into
        # the prepared adapter .lib and are intentionally absent from the
        # manifest's real-symbol list.
        iface_syms = set()
        if os.path.exists(iface):
            for line in open(iface, encoding="utf-8"):
                m = re.search(r'extern fn \S+\([^)]*\) -> \S+ "([^"]+)"', line)
                if m:
                    iface_syms.add(m.group(1))
        missing = [s for s in iface_syms if s not in symbols and not s.startswith("lime_")]
        if missing:
            print(f"  FAIL  {lib}: iface symbols not in manifest: {missing[:5]}")
            fails.append(lib)
            continue
        print(f"  PASS  {lib} (artifact={artifact_name}, {len(symbols)} symbols)")
    return fails


def main():
    if not os.path.exists(LIME_EXE):
        print(f"lime binary not found at {LIME_EXE}; run `cargo build --release` first")
        sys.exit(2)
    core_fails = check_core()
    store_fails = check_store_integrity()
    print()
    if not core_fails and not store_fails:
        print("ALL CHECKS PASSED")
        sys.exit(0)
    print(f"REGRESSION: core_fails={core_fails} store_fails={store_fails}")
    sys.exit(1)


if __name__ == "__main__":
    main()
