// Benchmark program generator for the Lime performance test suite.
//
// Produces six deterministic benchmark programs. Each lives in its own
// directory as `citrus.toml` + `main.lime` so they run through the same
// manifest entry path as the real compiler workflow (the `.lime`-direct
// entry point exercises a different, stricter path and is not used here).
//
// Benchmarks and the bottleneck they stress:
//   small                 -> baseline tiny program
//   medium                -> moderate func/stmt count, no generics/imports
//   large                 -> large stmt count, structs, light generics
//   generic_heavy         -> many distinct generic type instantiations
//   package_heavy         -> imports all stdlib packages + deep cross-pkg calls
//   monomorphization_heavy-> nested generics yielding many mono instances
//
// No language/spec/API/stdlib changes. Output is valid Lime source.
use std::fs::File;
use std::io::Write;

fn write(path: &str, body: &str) {
    let mut f = File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    println!("wrote {}", path);
}

fn manifest(name: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"v0.1.0\"\n\n[import]\ncollections = \"v0.1.0\"\ntime = \"v0.1.0\"\nfs = \"v0.1.0\"\nstring = \"v0.1.0\"\nmath = \"v0.1.0\"\n",
        name
    )
}

fn plain_manifest(name: &str) -> String {
    format!("[package]\nname = \"{}\"\nversion = \"v0.1.0\"\n", name)
}

fn small() -> String {
    let mut s = String::new();
    s.push_str("fn add(a: int, b: int) -> int { return a + b }\n");
    s.push_str("fn main():\n");
    s.push_str("    let x = add(1, 2)\n");
    s.push_str("    let y = add(x, 3)\n");
    s.push_str("    return\n");
    s
}

fn medium() -> String {
    let mut s = String::new();
    for i in 0..200 {
        s.push_str(&format!("fn f{} (a: int) -> int {{ return a + {} }}\n", i, i % 17));
    }
    s.push_str("fn main():\n");
    for i in 0..2000 {
        s.push_str(&format!("    let v{} = f{}({})\n", i, i % 200, i % 13));
    }
    s.push_str("    return\n");
    s
}

fn large() -> String {
    let mut s = String::new();
    s.push_str("struct Point { x: int, y: int }\n");
    s.push_str("fn mk(x: int, y: int) -> Point { let p = Point(); p.x = x; p.y = y; return p }\n");
    for i in 0..800 {
        s.push_str(&format!("fn g{} (a: int) -> int {{ return a * {} + {} }}\n", i, (i % 31) + 1, i % 7));
    }
    s.push_str("fn id<T>(x: T) -> T { return x }\n");
    s.push_str("fn main():\n");
    for i in 0..20000 {
        let f = i % 800;
        s.push_str(&format!("    let p{} = mk({} % 100, {} % 100)\n", i, i, i + 1));
        s.push_str(&format!("    let v{} = g{}(p{}.x + p{}.y)\n", i, f, i, i));
        s.push_str(&format!("    let _w{} = id({})\n", i, i % 23));
    }
    s.push_str("    return\n");
    s
}

fn generic_heavy() -> String {
    let mut s = String::new();
    s.push_str("fn id<T>(x: T) -> T { return x }\n");
    s.push_str("fn swap<T>(a: T, b: T) -> List<T> { let l: List<T> = List(); l.push(b); l.push(a); return l }\n");
    s.push_str("fn first<T>(l: List<T>) -> T { return l.get(0) }\n");
    s.push_str("fn len_<T>(l: List<T>) -> int { return l.size() }\n");
    s.push_str("fn get<K,V>(m: HashMap<K,V>, k: K) -> V { return m.get(k) }\n");
    s.push_str("fn main():\n");
    let prims = ["int", "float", "str", "bool"];
    let mut k = 0;
    for a in prims {
        for b in prims {
            for c in prims {
                s.push_str(&format!("    let _l{}: List<{}> = swap(id({}0), id({}0))\n", k, a, a, a));
                s.push_str(&format!("    let _m{}: int = len_(swap({}0, {}0))\n", k, b, b));
                s.push_str(&format!("    let _s{} = first(swap({}0, {}0))\n", k, c, c));
                k += 1;
            }
        }
    }
    for a in prims {
        for b in prims {
            s.push_str(&format!("    let _h{} = get(HashMap({}0, \"{}\"), {}0)\n", k, a, b, a));
            k += 1;
        }
    }
    s.push_str("    return\n");
    s
}

fn package_heavy() -> String {
    let mut s = String::new();
    s.push_str("fn lvl(n: int) -> int {\n    let l: List<int> = List()\n    l.push(n)\n    l.push(n + 1)\n    let s = string.from_int(n)\n    let t = time.now()\n    let m: HashMap<int,str> = HashMap(n, s)\n    let r = math.abs(n)\n    return l.size() + r + m.size()\n}\n");
    s.push_str("fn main():\n");
    for i in 0..4000 {
        s.push_str(&format!("    let a{} = lvl({})\n", i, i % 11));
    }
    s.push_str("    return\n");
    s
}

fn monomorphization_heavy() -> String {
    let mut s = String::new();
    s.push_str("fn box<T>(x: T) -> List<T> { let l: List<T> = List(); l.push(x); return l }\n");
    s.push_str("fn unbox<T>(l: List<T>) -> T { return l.get(0) }\n");
    s.push_str("fn chain<T>(x: T) -> T { return unbox(box(x)) }\n");
    s.push_str("fn twice<T>(x: T) -> List<T> { let a = chain(x); let b = chain(a); let l: List<T> = List(); l.push(a); l.push(b); return l }\n");
    s.push_str("fn main():\n");
    let prims = ["int", "float", "str", "bool"];
    let mut k = 0;
    for a in prims {
        for b in prims {
            for c in prims {
                for d in prims {
                    s.push_str(&format!("    let _t{} = twice(chain(box({}0)))\n", k, a));
                    s.push_str(&format!("    let _u{} = unbox(twice(chain(box({}0))))\n", k, b));
                    s.push_str(&format!("    let _w{} = chain(unbox(twice(box({}0))))\n", k, c));
                    s.push_str(&format!("    let _z{} = twice(twice(box({}0)))\n", k, d));
                    k += 1;
                }
            }
        }
    }
    s.push_str("    return\n");
    s
}

fn main() {
    let dir = "benchmarks/programs";
    std::fs::create_dir_all(dir).unwrap();

    // non-importing benchmarks use a plain manifest
    for (name, body) in [
        ("small", small()),
        ("medium", medium()),
        ("large", large()),
        ("generic_heavy", generic_heavy()),
        ("monomorphization_heavy", monomorphization_heavy()),
    ] {
        let p = format!("{}/{}", dir, name);
        std::fs::create_dir_all(&p).unwrap();
        write(&format!("{}/citrus.toml", p), &plain_manifest(name));
        write(&format!("{}/main.lime", p), &body);
    }

    // package-heavy imports all stdlib packages
    let p = format!("{}/package_heavy", dir);
    std::fs::create_dir_all(&p).unwrap();
    write(&format!("{}/citrus.toml", p), &manifest("package_heavy"));
    write(&format!("{}/main.lime", p), &package_heavy());
}
