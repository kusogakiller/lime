#!/usr/bin/env python3
"""Fair benchmark harness: Lime Native vs Clang.

For each benchmark named <name>:
  suite/<name>.lime   -> lime build --release --emit-object -> <name>_lime.exe
  suite/<name>.c      -> clang -O2 -c ... actually full link: clang -O2 -> <name>_clang_o2.exe
                         clang -O3 -> <name>_clang_o3.exe
Runs each: warmup (1) + repeats (N) via Windows high-resolution timer (time.perf_counter).
Records per-run wall time; computes min/median/max.

Output: results/<name>.json and results/README with environment.

Lime Native is pinned at -O2 (the compiler emits `clang -O2` for release), so the
primary fair comparison is Lime -O2 vs Clang -O2. Clang -O3 is reported for reference.
"""
import os, sys, subprocess, json, statistics, shutil, datetime

ROOT = os.path.dirname(os.path.abspath(__file__))
SUITE = os.path.join(ROOT, "suite")
RESULTS = os.path.join(ROOT, "results")
os.makedirs(RESULTS, exist_ok=True)

LIME_EXE = r"C:\Users\szzxl\Downloads\lime\target\release\lime.exe"
CLANG = r"C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin\clang.exe"
LLVM_BIN = r"C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin"

REPEATS = int(os.environ.get("BENCH_REPEATS", "11"))
WARMUP = 1

def run_capture(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)

def build_lime(name):
    # The Lime compiler emits <name>.exe / .ll / .obj NEXT TO THE SOURCE FILE.
    # We move the produced exe into RESULTS as <name>_lime.exe.
    src_exe = os.path.join(SUITE, f"{name}.exe")
    src_ll = os.path.join(SUITE, f"{name}.ll")
    src_obj = os.path.join(SUITE, f"{name}.obj")
    dst_exe = os.path.join(RESULTS, f"{name}_lime.exe")
    for p in (src_exe, dst_exe):
        if os.path.exists(p):
            os.remove(p)
    lime_src = os.path.join(SUITE, f"{name}.lime")
    r = run_capture([LIME_EXE, "build", "--release", "--emit-object", lime_src],
                    cwd=ROOT)
    ok = os.path.exists(src_exe)
    if not ok:
        return False, (r.stdout + r.stderr)[-600:]
    shutil.move(src_exe, dst_exe)
    # keep .ll/.obj for inspection
    if os.path.exists(src_ll):
        shutil.move(src_ll, os.path.join(RESULTS, f"{name}_lime.ll"))
    if os.path.exists(src_obj):
        shutil.move(src_obj, os.path.join(RESULTS, f"{name}_lime.obj"))
    return True, ""

def build_clang(name, opt):
    c_src = os.path.join(SUITE, f"{name}.c")
    exe = os.path.join(RESULTS, f"{name}_clang_o{opt}.exe")
    cmd = [CLANG, f"-O{opt}", "-o", exe, c_src]
    r = run_capture(cmd, cwd=ROOT, env={**os.environ, "PATH": os.environ["PATH"]})
    ok = os.path.exists(exe)
    return ok, (r.stderr if not ok else "")

def time_exe(exe, n, warmup):
    times = []
    for _ in range(warmup):
        subprocess.run([exe], capture_output=True, text=True)
    for _ in range(n):
        r = subprocess.run([exe], capture_output=True, text=True)
        # use perf_counter via python wall measurement using Start-Process? simpler: measure here
        # We measure the wall time of the subprocess itself:
    return times

def measure(exe, n, warmup):
    samples = []
    for i in range(warmup + n):
        import time
        t0 = time.perf_counter()
        r = subprocess.run([exe], capture_output=True, text=True)
        t1 = time.perf_counter()
        if r.returncode != 0:
            return None, f"rc={r.returncode} stderr={r.stderr[:200]}"
        if i >= warmup:
            samples.append((t1 - t0) * 1000.0)
    return samples, r.stdout.strip()

def summarize(samples):
    return {
        "min_ms": round(min(samples), 4),
        "median_ms": round(statistics.median(samples), 4),
        "max_ms": round(max(samples), 4),
        "mean_ms": round(statistics.mean(samples), 4),
        "stdev_ms": round(statistics.pstdev(samples), 4) if len(samples) > 1 else 0.0,
        "n": len(samples),
    }

def main():
    names = [a[:-5] for a in os.listdir(SUITE) if a.endswith(".lime")]
    # pair with .c
    report = {"benchmarks": {}}
    env = {
        "date": datetime.datetime.now().isoformat(timespec="seconds"),
        "cpu": "Intel x86_64 Family 6 Model 158 (Skylake-class), 8 logical cores",
        "os": "Windows 10.0.26200 (build 26200), MINGW64",
        "lime_exe": LIME_EXE,
        "lime_git": "4019b2a",
        "clang": "22.1.8 (LLVM 22.1.8)",
        "lime_flags": "lime build --release --emit-object  => clang -O2 -c + lld-link",
        "clang_o2_flags": "clang -O2 -o",
        "clang_o3_flags": "clang -O3 -o",
        "repeats": REPEATS, "warmup": WARMUP,
    }
    for name in sorted(names):
        cpath = os.path.join(SUITE, f"{name}.c")
        entry = {"has_c_ref": os.path.exists(cpath), "lime": None, "clang_o2": None, "clang_o3": None,
                 "correctness": None}
        lok, lerr = build_lime(name)
        if not lok:
            entry["lime"] = {"build": "FAIL", "err": lerr[-500:]}
        else:
            s, out = measure(os.path.join(RESULTS, f"{name}_lime.exe"), REPEATS, WARMUP)
            if s is None:
                entry["lime"] = {"build": "PASS", "run": "FAIL", "err": out}
            else:
                entry["lime"] = {"build": "PASS", "run": "PASS", **summarize(s), "output": out}
        if entry["has_c_ref"]:
            for opt in (2, 3):
                ok, err = build_clang(name, opt)
                if not ok:
                    entry[f"clang_o{opt}"] = {"build": "FAIL", "err": err[-500:]}
                else:
                    s, out = measure(os.path.join(RESULTS, f"{name}_clang_o{opt}.exe"), REPEATS, WARMUP)
                    if s is None:
                        entry[f"clang_o{opt}"] = {"build": "PASS", "run": "FAIL", "err": out}
                    else:
                        entry[f"clang_o{opt}"] = {"build": "PASS", "run": "PASS", **summarize(s), "output": out}
            if entry["lime"] and isinstance(entry["lime"], dict) and entry["lime"].get("output") is not None \
               and entry["clang_o2"] and isinstance(entry["clang_o2"], dict):
                lo = entry["lime"].get("output"); co = entry["clang_o2"].get("output")
                entry["correctness"] = "MATCH" if lo == co else f"MISMATCH lime={lo!r} clang={co!r}"
        report["benchmarks"][name] = entry
    with open(os.path.join(RESULTS, "benchmark_results.json"), "w") as f:
        json.dump({"environment": env, "results": report}, f, indent=2)
    print(json.dumps({"environment": env, "results": report}, indent=2))

if __name__ == "__main__":
    main()
