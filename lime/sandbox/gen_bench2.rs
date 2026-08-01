use std::fs::File;
use std::io::Write;
fn main() {
    let mut f = File::create("sandbox/bench2/main.lime").unwrap();
    writeln!(f, "import collections").unwrap();
    writeln!(f, "import time").unwrap();
    writeln!(f, "import fs").unwrap();
    writeln!(f, "import string").unwrap();
    writeln!(f, "import math").unwrap();
    writeln!(f).unwrap();
    
    writeln!(f, "fn id<T>(x: T) -> T {{ return x }}").unwrap();
    writeln!(f, "fn pair<T>(a: T, b: T) -> List<T> {{ let l: List<T> = List(); l.push(a); l.push(b); return l }}").unwrap();
    writeln!(f, "fn get<K,V>(m: HashMap<K,V>, k: K) -> V {{ return m.get(k) }}").unwrap();
    writeln!(f, "fn len_<T>(l: List<T>) -> int {{ return l.size() }}").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "fn main() {{").unwrap();
    for i in 0..8000 {
        writeln!(f, "    let a{}: int = id({})", i, i).unwrap();
        writeln!(f, "    let m{}: HashMap<int,str> = HashMap({}, \"v{}\")", i, i, i).unwrap();
        writeln!(f, "    let g{}: str = get(m{}, {})", i, i, i).unwrap();
        writeln!(f, "    let l{}: List<float> = pair({}.0, {}.5)", i, i, i).unwrap();
        writeln!(f, "    let n{}: int = len_(l{})", i, i).unwrap();
    }
    writeln!(f, "    print(id(1))").unwrap();
    writeln!(f, "    print(get(HashMap(1,\"x\"), 1))").unwrap();
    writeln!(f, "    print(len_(pair(1.0,2.0)))").unwrap();
    writeln!(f, "    print(a7999)").unwrap();
    writeln!(f, "    print(g7999)").unwrap();
    writeln!(f, "    print(n7999)").unwrap();
    writeln!(f, "}}").unwrap();
}