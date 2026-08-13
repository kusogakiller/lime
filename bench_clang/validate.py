#!/usr/bin/env python3
"""Validate every benchmark pair: build (Lime + Clang O2) and compare outputs.
No timing. Reports build/run status and correctness per pair. Stops are NOT forced;
it just lists mismatches. This is the audit gate before the timed run."""
import os, subprocess, json

ROOT = os.path.dirname(os.path.abspath(__file__))
SUITE = os.path.join(ROOT, "suite")
RESULTS = os.path.join(ROOT, "results")
os.makedirs(RESULTS, exist_ok=True)
LIME = r"C:\Users\szzxl\Downloads\lime\target\release\lime.exe"
CLANG = r"C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin\clang.exe"

def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True)

def main():
    names = sorted(a[:-5] for a in os.listdir(SUITE) if a.endswith(".lime"))
    report = {}
    for name in names:
        e = {"lime_build": None, "lime_out": None, "clang_build": None, "clang_out": None, "match": None, "note": ""}
        # Lime
        lime_src = os.path.join(SUITE, f"{name}.lime")
        src_exe = os.path.join(SUITE, f"{name}.exe")
        if os.path.exists(src_exe): os.remove(src_exe)
        r = run([LIME, "build", "--release", "--emit-object", lime_src])
        if os.path.exists(src_exe):
            e["lime_build"] = "PASS"
            rr = run([src_exe])
            e["lime_out"] = rr.stdout.strip()
            if rr.returncode != 0:
                e["lime_build"] = "RUNFAIL"; e["note"] = f"rc={rr.returncode}"
        else:
            e["lime_build"] = "FAIL"; e["note"] = (r.stdout + r.stderr)[-300:]
        # Clang
        c_src = os.path.join(SUITE, f"{name}.c")
        c_exe = os.path.join(RESULTS, f"{name}_clang_o2.exe")
        if os.path.exists(c_exe): os.remove(c_exe)
        if os.path.exists(c_src):
            rc = run([CLANG, "-O2", "-o", c_exe, c_src])
            if os.path.exists(c_exe):
                e["clang_build"] = "PASS"
                rr = run([c_exe])
                e["clang_out"] = rr.stdout.strip()
            else:
                e["clang_build"] = "FAIL"; e["note"] += " | clang:" + (rc.stdout+rc.stderr)[-300:]
        else:
            e["clang_build"] = "NO_C"
        if e["lime_out"] is not None and e["clang_out"] is not None:
            e["match"] = "MATCH" if e["lime_out"] == e["clang_out"] else f"MISMATCH lime={e['lime_out']!r} clang={e['clang_out']!r}"
        report[name] = e
    # print
    print(f"{'benchmark':18} {'lime':8} {'clang':8} {'match':10} note")
    for n, e in report.items():
        print(f"{n:18} {str(e['lime_build']):8} {str(e['clang_build']):8} {str(e['match']):10} {e['note'][:60]}")
    json.dump(report, open(os.path.join(RESULTS, "validation.json"), "w"), indent=2)

if __name__ == "__main__":
    main()
