"""Python-side dead-code analyzer for deadcode.

This module is embedded into the deadcode Rust binary via pyo3.
It provides language-aware symbol extraction for Python files.
"""
import ast
import re
from typing import List, Tuple


def extract_symbols(source: str) -> List[Tuple[int, str]]:
    """Return (line_number, name) for every top-level definition."""
    out: List[Tuple[int, str]] = []
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return out
    for node in tree.body:
        name = None
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            name = node.name
        elif isinstance(node, ast.ClassDef):
            name = node.name
        elif isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name):
                    name = target.id
        elif isinstance(node, ast.AnnAssign):
            if isinstance(node.target, ast.Name):
                name = node.target.id
        elif isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name:
                    out.append((node.lineno, alias.name.split('.')[0]))
        elif isinstance(node, ast.ImportFrom):
            for alias in node.names:
                if alias.name and alias.name != '*':
                    out.append((node.lineno, alias.name))
        if name and name != '_':
            out.append((node.lineno, name))
    return out


def count_references(source: str, symbol: str, def_line: int) -> int:
    """Count occurrences of `symbol` outside its definition line."""
    lines = source.split('\n')
    count = 0
    for i, line in enumerate(lines):
        if i + 1 == def_line:
            continue
        count += len(re.findall(r'\b' + re.escape(symbol) + r'\b', line))
    return count


def analyze(source: str) -> List[Tuple[int, str]]:
    """Return list of dead symbols (line, name) in a Python source."""
    symbols = extract_symbols(source)
    dead = []
    for ln, name in symbols:
        if count_references(source, name, ln) == 0:
            dead.append((ln, name))
    return dead


if __name__ == '__main__':
    import sys
    for path in sys.argv[1:]:
        with open(path) as f:
            src = f.read()
        for ln, name in analyze(src):
            print(f"DEAD  {path}:{ln}  {name}")
