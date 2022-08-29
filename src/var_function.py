def Var(k):
    try:
        return vars[k]
    except KeyError:
        return gclient_builtin_vars[k]
