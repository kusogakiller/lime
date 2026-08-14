#!/usr/bin/env python3
"""Measure all 16 benchmark categories: Lime Native vs Clang O2/O3.
For each category: build Lime exe (--emit-object), clang -O2, clang -O3,
run each N times, take median wall-clock, and report ratio (Lime/O3).
Correctness is assumed MATCH (validated separately by validate.py)."""
import subprocess, os, statistics, shutil, sys, json

ROOT = r"C:/Users/szzxl/Downloads/lime"
LIME = os.path.join(ROOT, "target", "release", "lime.exe")
B = os.path.join(ROOT, "bench_clang")
SUITE = os.path.join(B, "suite")
T = r"C:/Users/szzxl/AppData/Local/Temp"
CLANG = r"C:/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin/clang.exe"

CATEGORIES = [
    "algo_sieve", "algo_sort", "control_flow", "float_loop", "func_call",
    "int_loop", "list_iter", "list_push", "map_ops", "memory_alloc",
    "mixed_workload", "recursion_tail", "recursion_tree", "set_ops",
    "string_access", "string_concat", "struct_ops",
]

RUNS = 51

def run_exe(exe):
    try:
        r = subprocess.run([exe], capture_output=True, text=True, timeout=60)
        return r.returncode, r.stdout.strip()
    except Exception as e:
        return -1, str(e)

def measure(exe):
    times = []
    for _ in range(RUNS):
        import time
        t0 = time.perf_counter()
        rc, _ = run_exe(exe)
        t1 = time.perf_counter()
        if rc != 0:
            return None
        times.append(t1 - t0)
    return statistics.median(times)

def main():
    results = {}
    print(f"{'bench':<16} {'lime_ms':>9} {'o2_ms':>9} {'o3_ms':>9} {'lime/o3':>8} {'verdict':>8}")
    print("-" * 60)
    for cat in CATEGORIES:
        lime_src = os.path.join(SUITE, cat + ".lime")
        c_src = os.path.join(SUITE, cat + ".c")
        if not os.path.exists(lime_src):
            print(f"{cat:<16} NO_LIME_SRC"); continue
        # Build Lime
        lime_exe = os.path.join(SUITE, cat + ".exe")
        if os.path.exists(lime_exe): os.remove(lime_exe)
        bl = subprocess.run([LIME, "build", "--release", "--emit-object", lime_src],
                            capture_output=True, text=True)
        if not os.path.exists(lime_exe):
            print(f"{cat:<16} LIME_BUILD_FAIL: {bl.stdout.strip()[:50]}"); continue
        lime_ms = measure(lime_exe)
        # Clang O2
        o2_exe = os.path.join(T, cat + "_o2.exe")
        if os.path.exists(c_src):
            subprocess.run([CLANG, "-O2", "-o", o2_exe, c_src], capture_output=True, text=True)
            o2_ms = measure(o2_exe) if os.path.exists(o2_exe) else None
        else:
            o2_ms = None
        # Clang O3
        o3_exe = os.path.join(T, cat + "_o3.exe")
        if os.path.exists(c_src):
            subprocess.run([CLANG, "-O3", "-o", o3_exe, c_src], capture_output=True, text=True)
            o3_ms = measure(o3_exe) if os.path.exists(o3_exe) else None
        else:
            o3_ms = None
        if lime_ms is None or o3_ms is None:
            print(f"{cat:<16} {'%.2f'%(lime_ms*1000) if lime_ms else 'FAIL':>9} {'-':>9} {'-':>9} {'-':>8} {'INCOMPLETE':>8}")
            continue
        ratio = lime_ms / o3_ms
        verdict = "WIN" if ratio <= 1.0 else "LOSE"
        print(f"{cat:<16} {lime_ms*1000:9.2f} {o2_ms*1000:9.2f} {o3_ms*1000:9.2f} {ratio:8.3f} {verdict:>8}")
        results[cat] = {"lime_ms": lime_ms*1000, "o2_ms": (o2_ms*1000 if o2_ms else None),
                       "o3_ms": o3_ms*1000, "ratio": ratio, "verdict": verdict}
    # Summary
    wins = sum(1 for r in results.values() if r["verdict"] == "WIN")
    print("-" * 60)
    print(f"WINS: {wins}/{len(results)} categories (Lime < Clang O3)")
    with open(os.path.join(B, "results", "benchmark_results_phase4.json"), "w") as f:
        json.dump(results, f, indent=2)
    print("Saved results/measure_all_phase4.json")

if __name__ == "__main__":
    main()
