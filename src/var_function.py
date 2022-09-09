def Var(k):
    v = gclient_custom_vars.get(k)
    if v is not None:
        return v
    v = vars.get(k)
    if v is not None:
        return v
    v = gclient_builtin_vars.get(k)
    if v is not None:
        return v
    raise Exception(f'Var("{k}") unresolved')
