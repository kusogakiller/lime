#!/usr/bin/env python3
"""Aggregate benchmark_results.json into the classification table + summary.

Classification thresholds (policy §11):
  ratio = Lime_median / Clang_O2_median
  < 0.90   -> Lime significantly faster
  0.90-0.98 -> Lime faster
  0.98-1.02 -> approximately equal
  1.02-1.10 -> Lime slower
  > 1.10   -> Lime significantly slower
Prints a Markdown table and tallies. Reads bench_clang/results/benchmark_results.json.
"""
import os, json, sys

ROOT = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(ROOT, "results", "benchmark_results.json")

def classify(ratio):
    if ratio < 0.90: return "Lime significantly faster"
    if ratio < 0.98: return "Lime faster"
    if ratio <= 1.02: return "Approximately equal"
    if ratio <= 1.10: return "Lime slower"
    return "Lime significantly slower"

def main():
    if not os.path.exists(RES):
        print("NO RESULTS FILE:", RES); sys.exit(2)
    data = json.load(open(RES))
    env = data["environment"]
    results = data["results"]["benchmarks"]
    rows = []
    cats = {}
    for name, b in results.items():
        lime = b.get("lime"); c2 = b.get("clang_o2"); c3 = b.get("clang_o3")
        if not (isinstance(lime, dict) and lime.get("run") == "PASS" and isinstance(c2, dict) and c2.get("run") == "PASS"):
            rows.append((name, "NOT COMPARABLE", "-", lime, c2, b.get("correctness"),
                         b.get("lime", {}).get("build") if isinstance(b.get("lime"), dict) else b.get("lime"),
                         c2.get("build") if isinstance(c2, dict) else c2))
            continue
        ratio = lime["median_ms"] / c2["median_ms"]
        cls = classify(ratio)
        rows.append((name, cls, round(ratio, 3), lime, c2, b.get("correctness"), "PASS", "PASS"))
    # print
    print(f"# Benchmark aggregation  (generated from {os.path.basename(RES)})")
    print(f"Date: {env.get('date')} | Lime git {env.get('lime_git')} | Clang {env.get('clang')}")
    print()
    print("| Benchmark | Classification | Lime/Clang-O2 | Lime median(ms) | Clang-O2 median(ms) | Clang-O3 median(ms) | Correctness |")
    print("|-----------|----------------|---------------|-----------------|---------------------|---------------------|-------------|")
    for name, cls, ratio, lime, c2, corr, lb, cb in rows:
        lm = f"{lime['median_ms']}" if isinstance(lime, dict) and 'median_ms' in lime else "-"
        c2m = f"{c2['median_ms']}" if isinstance(c2, dict) and 'median_ms' in c2 else "-"
        c3blob = results.get(name, {}).get("clang_o3")
        c3m = f"{c3blob['median_ms']}" if isinstance(c3blob, dict) and 'median_ms' in c3blob else "-"
        print(f"| {name} | {cls} | {ratio} | {lm} | {c2m} | {c3m} | {corr} |")
    # tallies
    from collections import Counter
    tally = Counter(r[1] for r in rows)
    print()
    print("## Tally")
    for k in ["Lime significantly faster","Lime faster","Approximately equal","Lime slower","Lime significantly slower","NOT COMPARABLE"]:
        if tally.get(k): print(f"- {k}: {tally[k]}")
    # wins/losses
    lime_wins = tally.get("Lime significantly faster",0)+tally.get("Lime faster",0)
    clang_wins = tally.get("Lime significantly slower",0)+tally.get("Lime slower",0)
    equal = tally.get("Approximately equal",0)
    print()
    print(f"Lime wins (faster/signif): {lime_wins}")
    print(f"Clang wins (slower/signif): {clang_wins}")
    print(f"Approximately equal: {equal}")
    print(f"Not comparable: {tally.get('NOT COMPARABLE',0)}")
    # extremes
    comp = [(r[0], r[2], r[1]) for r in rows if isinstance(r[2], (int,float))]
    if comp:
        best = min(comp, key=lambda x: x[1])
        worst = max(comp, key=lambda x: x[1])
        print()
        print(f"Largest Lime advantage: {best[0]} ({best[1]}x, {best[2]})")
        print(f"Largest Lime disadvantage: {worst[0]} ({worst[1]}x, {worst[2]})")

if __name__ == "__main__":
    main()
