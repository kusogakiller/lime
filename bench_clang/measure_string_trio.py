#!/usr/bin/env python3
"""Measure only the string trio with the current (OPT-01) Lime compiler and
compare against Clang O2/O3. Prints Before (frozen baseline) vs After."""
import os, sys, subprocess, json, statistics, time

ROOT = os.path.dirname(os.path.abspath(__file__))
SUITE = os.path.join(ROOT, "suite")
RESULTS = os.path.join(ROOT, "results")
LIME = r"C:\Users\szzxl\Downloads\lime\target\release\lime.exe"
CLANG = r"C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin\clang.exe"
REPEATS = 11
WARMUP = 1

def build_lime(name):
    src_exe = os.path.join(SUITE, f"{name}.exe")
    if os.path.exists(src_exe): os.remove(src_exe)
    r = subprocess.run([LIME, "build", "--release", "--emit-object", os.path.join(SUITE, f"{name}.lime")],
                       capture_output=True, text=True, cwd=ROOT)
    dst = os.path.join(RESULTS, f"{name}_lime.exe")
    if os.path.exists(src_exe):
        if os.path.exists(dst): os.remove(dst)
        os.rename(src_exe, dst)
        return True, dst, ""
    return False, None, (r.stdout+r.stderr)[-400:]

def build_clang(name, opt):
    exe = os.path.join(RESULTS, f"{name}_clang_o{opt}.exe")
    if os.path.exists(exe): os.remove(exe)
    r = subprocess.run([CLANG, f"-O{opt}", "-o", exe, os.path.join(SUITE, f"{name}.c")],
                       capture_output=True, text=True, cwd=ROOT)
    return os.path.exists(exe), exe, (r.stdout+r.stderr)[-400:]

def measure(exe, n, warmup):
    samples=[]
    out=None
    for i in range(warmup+n):
        t0=time.perf_counter()
        r=subprocess.run([exe], capture_output=True, text=True)
        t1=time.perf_counter()
        if r.returncode!=0: return None, r.stderr[:200]
        if i>=warmup: samples.append((t1-t0)*1000.0)
        out=r.stdout.strip()
    return samples, out

def summ(s): return round(statistics.median(s),4)

names=["string_access","string_concat","mixed_workload"]
frozen=json.load(open(os.path.join(RESULTS,"benchmark_results.frozen_baseline.json")))["results"]["benchmarks"]

print(f"{'bench':16} {'B_lime':>10} {'A_lime':>10} {'O2':>10} {'O3':>10} {'A/O3':>7} match")
for n in names:
    lok, lexe, lerr = build_lime(n)
    if not lok:
        print(f"{n:16} build FAIL: {lerr}")
        continue
    ls, lout = measure(lexe, REPEATS, WARMUP)
    if ls is None:
        print(f"{n:16} run FAIL: {lout}")
        continue
    c2ok, c2e, _ = build_clang(n,2); c3ok, c3e, _ = build_clang(n,3)
    c2s, c2out = measure(c2e, REPEATS, WARMUP) if c2ok else (None,None)
    c3s, c3out = measure(c3e, REPEATS, WARMUP) if c3ok else (None,None)
    bl = frozen.get(n,{}).get("lime",{}).get("median_ms")
    al = summ(ls)
    o2 = summ(c2s) if c2s else None
    o3 = summ(c3s) if c3s else None
    ratio = (al/o3) if o3 else float('nan')
    match = "MATCH" if (lout is not None and c2out is not None and lout==c2out) else "MISMATCH"
    print(f"{n:16} {str(bl):>10} {al:>10} {str(o2):>10} {str(o3):>10} {ratio:>7.3f} {match}")
