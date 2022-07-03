class str(__builtins__.str):
    def _is_wrapped(self):
        return True

    def __bool__(self):
        glob = globals()
        value = eval(self, glob, glob['vars'])
        if isinstance(value, __builtins__.str):
            return value.__bool__()
        return value
