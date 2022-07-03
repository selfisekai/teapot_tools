class str(__builtins__.str):
    def _is_wrapped(self):
        return True

    def __bool__(self):
        value = eval(self)
        if isinstance(value, __builtins__.str):
            return str(value).__bool__()
        return value
