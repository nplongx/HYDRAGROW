#!/usr/bin/env python3
"""Exercise the deterministic evidence comparator with pass and expected-reject cases."""
import operator

OPS = {"=": operator.eq, "!=": operator.ne, "<": operator.lt, "<=": operator.le, ">": operator.gt, ">=": operator.ge}


def evaluate(actual, op, target, unit, expected_unit):
    if unit != expected_unit:
        return False
    return OPS[op](actual, target)


def assert_rejected(actual, op, target, unit, expected_unit):
    # A failing acceptance is an expected result of the self-test, not a failing CI step.
    assert not evaluate(actual, op, target, unit, expected_unit)


def main():
    # Positive case: valid evidence must pass.
    assert evaluate(6.4, "<=", 10, "seconds", "seconds")

    # Negative cases: invalid evidence must be rejected by the evaluator.
    assert_rejected(13.7, "<=", 10, "seconds", "seconds")
    assert_rejected(6.4, "<=", 10, "milliseconds", "seconds")

    # Fail closed on an invalid operator instead of silently accepting it.
    try:
        evaluate(6.4, "~", 10, "seconds", "seconds")
    except KeyError:
        pass
    else:
        raise AssertionError("unsupported operator was not rejected")

    print("Governance self-test PASS: valid evidence passes; invalid evidence is rejected")


if __name__ == "__main__":
    main()
