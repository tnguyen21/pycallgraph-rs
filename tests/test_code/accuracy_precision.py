"""Accuracy fixture: precision / expected-absence tests.

These test cases verify that the analyzer does NOT produce spurious edges.
Each function is designed so a naive analysis might over-approximate, but
a correct analysis should be precise.
"""


# --- Higher-order isolation ---

def helper_a():
    pass

def helper_b():
    pass

def factory_a():
    return helper_a

def factory_b():
    return helper_b

def call_factory_a():
    """Calls factory_a() then the result — should reach helper_a, NOT helper_b."""
    fn = factory_a()
    fn()

def call_factory_b():
    """Calls factory_b() then the result — should reach helper_b, NOT helper_a."""
    fn = factory_b()
    fn()


# --- Inheritance method isolation ---

class Parent:
    def action(self):
        pass

class ChildA(Parent):
    def action(self):
        pass

class ChildB(Parent):
    def action(self):
        pass

def call_child_a_action():
    """Constructs ChildA and calls action — should NOT reach ChildB.action or Parent.action."""
    obj = ChildA()
    obj.action()

def call_child_b_action():
    """Constructs ChildB and calls action — should NOT reach ChildA.action or Parent.action."""
    obj = ChildB()
    obj.action()


# --- Decorator scope isolation ---

def decorator_one(f):
    return f

def decorator_two(f):
    return f

@decorator_one
def decorated_fn_one():
    pass

@decorator_two
def decorated_fn_two():
    pass

def call_only_one():
    """Calls decorated_fn_one — should NOT produce edge to decorator_two or decorated_fn_two."""
    decorated_fn_one()

def call_only_two():
    """Calls decorated_fn_two — should NOT produce edge to decorator_one or decorated_fn_one."""
    decorated_fn_two()


# --- Closure isolation ---

def outer_a():
    def inner():
        pass
    return inner()

def outer_b():
    def inner():
        pass
    return inner()

def call_outer_a():
    """Calls outer_a — should NOT reach outer_b or outer_b.<locals>.inner."""
    outer_a()

def call_outer_b():
    """Calls outer_b — should NOT reach outer_a or outer_a.<locals>.inner."""
    outer_b()


# --- Multi-return: unrelated class should not leak ---

class Cat:
    def speak(self):
        pass

class Dog:
    def speak(self):
        pass

class Fish:
    """Fish is never returned by choose_pet — should not appear in caller edges."""
    def speak(self):
        pass

def choose_pet(flag):
    if flag:
        return Cat()
    return Dog()

def call_pet_speak(flag):
    """Calls choose_pet().speak() — should reach Cat.speak and Dog.speak, NOT Fish.speak."""
    pet = choose_pet(flag)
    pet.speak()
