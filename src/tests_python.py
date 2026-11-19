"""Python tests for the deadcode Python analyzer."""
import sys
sys.path.insert(0, '.')
from python_analyzer import extract_symbols, analyze, count_references


def test_extract_symbols():
    src = "def foo():\n    pass\n\nclass Bar:\n    pass\n"
    syms = extract_symbols(src)
    names = [n for _, n in syms]
    assert "foo" in names
    assert "Bar" in names


def test_analyze_finds_dead():
    src = "def dead_fn():\n    pass\n\ndef used_fn():\n    pass\n\nused_fn()\n"
    dead = analyze(src)
    names = [n for _, n in dead]
    assert "dead_fn" in names
    assert "used_fn" not in names


def test_count_references():
    src = "x = 1\nx = 2\n"
    assert count_references(src, "x", 1) == 1  # line 2 references x


if __name__ == "__main__":
    test_extract_symbols()
    test_analyze_finds_dead()
    test_count_references()
    print("All Python tests passed.")
