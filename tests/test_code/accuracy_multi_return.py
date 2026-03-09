"""Accuracy fixture: multi-return function propagation.

Covers:
  - choose(flag) returns either an A or B instance depending on the branch.
  - A sound analysis must keep BOTH candidate return types reachable at
    the call site, so both A.method and B.method appear as used.
"""


class A:
    def method(self):
        pass


class B:
    def method(self):
        pass


def choose(flag):
    """Returns an A or B depending on flag — two distinct return types."""
    if flag:
        return A()
    else:
        return B()


def wrap(flag):
    """Returns choose(flag) unchanged — nested return propagation must stay precise."""
    return choose(flag)


def multi_return_caller(flag):
    """Calls choose() and invokes .method() — both A.method and B.method must be reachable."""
    obj = choose(flag)
    obj.method()


def wrapped_multi_return_caller(flag):
    """Calls wrap() then .method() — nested return propagation must preserve both classes."""
    wrap(flag).method()
