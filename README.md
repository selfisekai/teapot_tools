# teapot_tools

reimplementation of gclient in Rust (or as some may call it, Python with extra steps).

## why?

I was trying to get depot_tools working on my machine, only to see things like this:

```
./third_party/depot_tools/bootstrap_python3: line 28: cipd: command not found
./third_party/depot_tools/bootstrap_python3: line 32: bootstrap-2@3.8.10.chromium.23_bin/python3/bin/python3: No such file or directory
/home/lauren/aports/testing/signal-desktop/src/webrtc-4896c/third_party/depot_tools/bootstrap_python3: line 28: cipd: command not found
/home/lauren/aports/testing/signal-desktop/src/webrtc-4896c/third_party/depot_tools/bootstrap_python3: line 32: bootstrap-2@3.8.10.chromium.23_bin/python3/bin/python3: No such file or directory
cat: can't open '/home/lauren/aports/testing/signal-desktop/src/webrtc-4896c/third_party/depot_tools/python3_bin_reldir.txt': No such file or directory
[E2022-06-21T19:47:00.253814+02:00 28299 0 annotate.go:273] goroutine 1:
#0 go.chromium.org/luci/vpython/venv/config.go:309 - venv.(*Config).resolvePythonInterpreter()
  reason: none of [/home/lauren/aports/testing/signal-desktop/src/webrtc-4896c/third_party/depot_tools//python3] matched specification 3.8.0

#1 go.chromium.org/luci/vpython/venv/config.go:153 - venv.(*Config).resolveRuntime()
  reason: failed to resolve system Python interpreter

#2 go.chromium.org/luci/vpython/venv/venv.go:143 - venv.With()
  reason: failed to resolve python runtime

#3 go.chromium.org/luci/vpython/run.go:60 - vpython.Run()
#4 go.chromium.org/luci/vpython/application/application.go:327 - application.(*application).mainImpl()
#5 go.chromium.org/luci/vpython/application/application.go:416 - application.(*Config).Main.func1()
#6 go.chromium.org/luci/vpython/application/support.go:46 - application.run()
#7 go.chromium.org/luci/vpython/application/application.go:415 - application.(*Config).Main()
#8 vpython/main.go:112 - main.mainImpl()
#9 vpython/main.go:118 - main.main()
#10 runtime/proc.go:225 - runtime.main()
#11 runtime/asm_amd64.s:1371 - runtime.goexit()
```

I looked at these errors, at the DEPS file, and got the impression that it's gonna be easier to rewrite it than to get it running.

## security considerations

**Do NOT run on untrusted projects.**

- DEPS file is literally Python code run on a Python interpreter, it possibly can run some malicious code.
- DEPS file contains hooks that are supposed to (not implemented yet) be run before/after cloning dependencies, they can be malicious code.

Please only report security issues over GitLab _with the confidentiality checkmark_, or by e-mail: `security at selfisekai dot rocks` (PGP key: [`A16F3AB139AEE4A3635D19ED734C629FD04BD319`](https://keys.selfisekai.rocks/gpg)).
