class str(__builtins__.str):
    def __bool__(self):
        value = eval(self)
        if isinstance(value, __builtins__.str):
            return str(value).__bool__()
        return value
