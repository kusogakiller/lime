import json, sys
p = sys.argv[1]
d = json.load(open(p))
def walk(n):
    if n.get('kind') == 'FunctionDecl':
        print('FUNC', n.get('name'), 'type=', json.dumps(n.get('type'))[:500])
    for c in n.get('inner', []):
        walk(c)
walk(d)
