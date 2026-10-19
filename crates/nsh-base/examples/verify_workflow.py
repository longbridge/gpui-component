"""Verify prebuilt Base runners through NSH's standard macOS run/package CLI.

This follows the Component adapter's runtime workflow. It never invokes Cargo,
changes the installed runtime, or adds hooks to the original TSX application.
"""
import json
import os
import pathlib
import queue
import shutil
import signal
import subprocess
import sys
import threading
import time

ROOT = pathlib.Path(__file__).resolve().parents[3]
ARTIFACTS = ROOT / "target/nsh-base-runtime-workflow"
RUNTIME = ARTIFACTS / "runtime"
FIXTURE = ARTIFACTS / "fixture"
BINARIES = pathlib.Path(os.environ.get("NSH_BASE_BINARIES", ROOT / "crates/nsh-base/target/debug"))
CLI = RUNTIME / "gpui-kit-nsh-base"
LOG = ARTIFACTS / "verification.log"
# These variables belong only to our child processes; the installed runtime and
# the caller's environment remain unchanged.
ENV = {key: value for key, value in os.environ.items() if key != "NSH_APP_DIR"}
ENV["NSH_RUNTIME"] = str(RUNTIME)


def output(message):
    print(message, flush=True)
    with LOG.open("a") as log:
        log.write(message + "\n")


def run(arguments):
    result = subprocess.run(arguments, env=ENV, cwd=ROOT, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            timeout=90)
    output(result.stdout)
    if result.returncode:
        raise RuntimeError(f"command failed: {arguments}: {result.returncode}")


def start(arguments, cwd=ROOT):
    child = subprocess.Popen(arguments, env=ENV, cwd=cwd, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                             start_new_session=True)
    lines = queue.Queue()
    lines.seen = []

    def collect():
        for line in child.stdout:
            lines.put(line.rstrip())

    threading.Thread(target=collect, daemon=True).start()
    return child, lines


def check_line(line):
    output(line)
    if any(error in line for error in ("panicked", "unknown native component", "NSH: ", "ReferenceError", "TypeError")):
        raise RuntimeError(line)


def await_marker(child, lines, marker):
    if any(marker in line for line in lines.seen):
        return
    deadline = time.monotonic() + 40
    while time.monotonic() < deadline:
        if child.poll() is not None:
            raise RuntimeError(f"child exited early: {child.returncode}")
        try:
            line = lines.get(timeout=.1)
        except queue.Empty:
            continue
        check_line(line)
        lines.seen.append(line)
        if marker in line:
            return
    raise RuntimeError(f"timed out awaiting {marker}")


def observe_running(child, lines, duration=2):
    deadline = time.monotonic() + duration
    while time.monotonic() < deadline:
        if child.poll() is not None:
            raise RuntimeError(f"runner exited during initialization: {child.returncode}")
        try:
            check_line(lines.get(timeout=.1))
        except queue.Empty:
            pass


def stop(child):
    # Every process we start has its own session. Never signal other NSH runs.
    try:
        os.killpg(child.pid, signal.SIGINT)
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGTERM)
        child.wait(timeout=5)


def main():
    if sys.platform != "darwin":
        raise RuntimeError("this package-resource verification covers macOS only")
    (RUNTIME / "gpui").mkdir(parents=True, exist_ok=True)
    FIXTURE.mkdir(parents=True, exist_ok=True)
    LOG.write_text("")
    for name in ("nsh-gpui-host", "nsh-gpui-app"):
        shutil.copy2(BINARIES / name, RUNTIME / "gpui" / name)
    shutil.copy2(BINARIES / CLI.name, CLI)
    for name in ("App.tsx", "nsh.json", "tsconfig.json"):
        shutil.copy2(ROOT / "crates/nsh-base/examples/app" / name, FIXTURE / name)
    output("Using fresh prebuilt Base CLI and NSH protocol runners in an isolated runtime")
    run([str(CLI), "check", str(FIXTURE)])
    declarations = (FIXTURE / ".nsh/types/nsh/ui/base.d.ts").read_text()
    for name in ("Button", "Checkbox", "Input"):
        assert f"export declare const {name}:" in declarations, name
    output("PASS: standard CLI generated and checked nsh/ui/base declarations for the original sample")

    child, lines = start([str(CLI), "-v", "run", str(FIXTURE), "--target", "gpui", "--port",
                          os.environ.get("NSH_BASE_VERIFY_PORT", "18885")])
    try:
        await_marker(child, lines, "loading build 1")
        observe_running(child, lines)
        source = (FIXTURE / "App.tsx").read_text()
        assert "Add a task to your workspace." in source
        (FIXTURE / "App.tsx").write_text(source.replace("Add a task to your workspace.",
                                                        "Add a task to your workspace. Updated."))
        await_marker(child, lines, "updating to build 2")
        await_marker(child, lines, "hot update applied")
        observe_running(child, lines)
        output("PASS: standard run loaded the original sample; the real backend accepted its source hot update without runtime errors")
    finally:
        stop(child)

    run([str(CLI), "package", str(FIXTURE), "--target", "gpui", "-o", str(ARTIFACTS / "package")])
    metadata = json.loads((FIXTURE / "nsh.json").read_text())
    package = ARTIFACTS / "package" / (metadata["name"] + ".app")
    executable = "".join(char for char in metadata["name"] if char.isascii() and char.isalnum())
    runner = package / "Contents/MacOS" / executable
    resources = package / "Contents/Resources/nsh"
    assert (resources / "bundle.nshc").is_file()
    assert (resources / "app.json").is_file()
    assert runner.is_file()
    child, lines = start([str(runner)], pathlib.Path("/tmp"))
    try:
        observe_running(child, lines, duration=3)
        output("PASS: packaged Base runner started from /tmp and remained running without resource or engine errors")
    finally:
        stop(child)

    # A negative control proves the installed runner reads its package resources,
    # rather than silently using a build-tree bundle or the current directory.
    bundle = resources / "bundle.nshc"
    hidden = resources / "bundle.nshc.verify-backup"
    bundle.rename(hidden)
    try:
        result = subprocess.run([str(runner)], env=ENV, cwd="/tmp", text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=20)
        output(result.stdout)
        assert result.returncode != 0, "missing package bundle must fail"
        assert "NSH:" in result.stdout, "missing package bundle must report its loading error"
        output("PASS: removing only the package bundle makes the same runner reject startup; the resource was restored")
    finally:
        hidden.rename(bundle)
    output("Validation: macOS only; CLI, HMR engine application and package resource loading. GUI interactions and task-result text are verified separately by task_app.")


if __name__ == "__main__":
    main()
