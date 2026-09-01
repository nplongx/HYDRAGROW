#!/usr/bin/env python3
"""Exercise the deterministic evidence comparator with one passing and one failing fixture."""
import operator

OPS = {"=": operator.eq, "!=": operator.ne, "<": operator.lt, "<=": operator.le, ">": operator.gt, ">=": operator.ge}


def evaluate(actual, op, target, unit, expected_unit):
    if unit != expected_unit:
        return False
    return OPS[op](actual, target)


def main():
    assert evaluate(6.4, "<=", 10, "seconds", "seconds")
    assert not evaluate(13.7, "<=", 10, "seconds", "seconds")
    print("Governance self-test PASS: valid evidence passes and invalid evidence is rejected")


if __name__ == "__main__":
    main()
