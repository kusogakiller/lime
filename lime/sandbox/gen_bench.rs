use std::fs::File;
use std::io::Write;
fn main() {
    let mut f = File::create("sandbox/bench/main.lime").unwrap();
    writeln!(f, "import collections").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "fn id<T>(x: T) -> T {{ return x }}").unwrap();
    writeln!(f, "fn pair<T>(a: T, b: T) -> List<T> {{ let l: List<T> = List(); l.push(a); l.push(b); return l }}").unwrap();
    writeln!(f, "fn map_get<K,V>(m: HashMap<K,V>, k: K) -> V {{ return m.get(k) }}").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "fn main() {{").unwrap();
    writeln!(f, "    let a: int = id(1)").unwrap();
    writeln!(f, "    let b: float = id(2.0)").unwrap();
    writeln!(f, "    let c: str = id(\"hi\")").unwrap();
    writeln!(f, "    let p1: List<int> = pair(1, 2)").unwrap();
    writeln!(f, "    let p2: List<float> = pair(3.0, 4.0)").unwrap();
    writeln!(f, "    let h: HashMap<int,str> = HashMap(1, \"x\")").unwrap();
    writeln!(f, "    let v: str = map_get(h, 1)").unwrap();
    for i in 0..20000 {
        writeln!(f, "    let t{}: int = id({})", i, i).unwrap();
        writeln!(f, "    let hh{}: HashMap<int,str> = HashMap({}, \"z\")", i, i).unwrap();
        writeln!(f, "    let g{}: str = map_get(hh{}, {})", i, i, i).unwrap();
    }
    writeln!(f, "    print(a)").unwrap();
    writeln!(f, "    print(b)").unwrap();
    writeln!(f, "    print(c)").unwrap();
    writeln!(f, "    print(p1.size())").unwrap();
    writeln!(f, "    print(v)").unwrap();
    writeln!(f, "    print(t3999)").unwrap();
    writeln!(f, "    print(g3999)").unwrap();
    writeln!(f, "}}").unwrap();
}
