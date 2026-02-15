#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="mock"
if [[ "${1:-}" == "--with-openai" ]]; then
  MODE="openai"
fi

timestamp="$(date +%Y%m%d-%H%M%S)"
kernel_log_done="logs/eval-runner-kernel-done-${timestamp}.jsonl"
kernel_log_cap="logs/eval-runner-kernel-cap-${timestamp}.jsonl"
tmp_apply_dir="$(mktemp -d /tmp/rx-eval-runner-applypatch.XXXXXX)"

declare -a PASSED=()
declare -a FAILED=()
declare -a SKIPPED=()

pass() {
  PASSED+=("$1")
  printf '[PASS] %s\n' "$1"
}

fail() {
  FAILED+=("$1")
  printf '[FAIL] %s\n' "$1"
}

skip() {
  SKIPPED+=("$1")
  printf '[SKIP] %s\n' "$1"
}

run_rx() {
  if [[ "$MODE" == "openai" ]]; then
    direnv exec . cargo run --bin rx -- "$@"
  else
    env -u OPENAI_API_KEY -u OPENAI_MODEL cargo run --bin rx -- "$@"
  fi
}

echo "== rx eval runner =="
echo "mode: $MODE"

if cargo build >/tmp/rx-eval-runner-build.out 2>/tmp/rx-eval-runner-build.err; then
  pass "cargo build"
else
  fail "cargo build"
fi

if cargo test >/tmp/rx-eval-runner-test.out 2>/tmp/rx-eval-runner-test.err; then
  pass "cargo test"
else
  fail "cargo test"
fi

if [[ "$MODE" == "openai" ]]; then
  if direnv exec . sh -lc '[ -n "${OPENAI_API_KEY:-}" ]'; then
    pass "OPENAI_API_KEY available via direnv"
  else
    fail "OPENAI_API_KEY available via direnv"
  fi
fi

if run_rx --debug-log "$kernel_log_done" "eval kernel loop runner" >/tmp/rx-eval-runner-k1.out 2>/tmp/rx-eval-runner-k1.err; then
  if rg -q '"type":"termination".*"reason":"done"|\"reason\":\"done\".*\"type\":\"termination\"' "$kernel_log_done"; then
    pass "kernel loop terminates via done"
  else
    fail "kernel loop terminates via done"
  fi
else
  fail "kernel loop run (done path)"
fi

if run_rx --max-iterations 1 --debug-log "$kernel_log_cap" "eval kernel cap runner" >/tmp/rx-eval-runner-k2.out 2>/tmp/rx-eval-runner-k2.err; then
  if rg -q '"type":"termination".*"reason":"max_iterations"|\"reason\":\"max_iterations\".*\"type\":\"termination\"' "$kernel_log_cap"; then
    pass "kernel enforces max-iterations cap"
  else
    fail "kernel enforces max-iterations cap"
  fi
else
  fail "kernel loop run (max-iterations path)"
fi

if rg -n 'registry\.register\(Arc::new\(ExecTool\)\);' src/main.rs >/dev/null \
  && rg -n 'registry\.register\(Arc::new\(ReadFileTool\)\);' src/main.rs >/dev/null \
  && rg -n 'registry\.register\(Arc::new\(WriteFileTool\)\);' src/main.rs >/dev/null \
  && rg -n 'registry\.register\(Arc::new\(ListDirTool\)\);' src/main.rs >/dev/null \
  && rg -n 'registry\.register\(Arc::new\(DoneTool\)\);' src/main.rs >/dev/null; then
  pass "tool registry includes minimal required tools"
else
  fail "tool registry includes minimal required tools"
fi

if rg -n 'not registered' src/kernel.rs >/dev/null; then
  pass "unknown tool path returns structured error"
else
  fail "unknown tool path returns structured error"
fi

if cargo build --bin apply_patch >/tmp/rx-eval-runner-ap-build.out 2>/tmp/rx-eval-runner-ap-build.err; then
  pass "cargo build --bin apply_patch"
else
  fail "cargo build --bin apply_patch"
fi

apply_bin="$ROOT_DIR/target/debug/apply_patch"
if [[ ! -x "$apply_bin" ]]; then
  fail "apply_patch binary exists"
else
  pass "apply_patch binary exists"
fi

if [[ -x "$apply_bin" ]]; then
  cd "$tmp_apply_dir"

  cat > add.patch <<'PATCH'
*** Begin Patch
*** Add File: a.txt
+hello
*** End Patch
PATCH
  if "$apply_bin" < add.patch >/tmp/rx-eval-runner-ap-1.out 2>/tmp/rx-eval-runner-ap-1.err \
    && [[ "$(cat a.txt)" == "hello" ]]; then
    pass "apply_patch add file"
  else
    fail "apply_patch add file"
  fi

  cat > update.patch <<'PATCH'
*** Begin Patch
*** Update File: a.txt
@@
-hello
+hello world
*** End Patch
PATCH
  if "$apply_bin" < update.patch >/tmp/rx-eval-runner-ap-2.out 2>/tmp/rx-eval-runner-ap-2.err \
    && [[ "$(cat a.txt)" == "hello world" ]]; then
    pass "apply_patch update file"
  else
    fail "apply_patch update file"
  fi

  cat > move.patch <<'PATCH'
*** Begin Patch
*** Update File: a.txt
*** Move to: moved.txt
@@
-hello world
+hello moved
*** End Patch
PATCH
  if "$apply_bin" < move.patch >/tmp/rx-eval-runner-ap-3.out 2>/tmp/rx-eval-runner-ap-3.err \
    && [[ ! -e a.txt ]] && [[ "$(cat moved.txt)" == "hello moved" ]]; then
    pass "apply_patch move file"
  else
    fail "apply_patch move file"
  fi

  cat > delete.patch <<'PATCH'
*** Begin Patch
*** Delete File: moved.txt
*** End Patch
PATCH
  if "$apply_bin" < delete.patch >/tmp/rx-eval-runner-ap-4.out 2>/tmp/rx-eval-runner-ap-4.err \
    && [[ ! -e moved.txt ]]; then
    pass "apply_patch delete file"
  else
    fail "apply_patch delete file"
  fi

  cat > abs.patch <<'PATCH'
*** Begin Patch
*** Add File: /tmp/x
+bad
*** End Patch
PATCH
  if "$apply_bin" < abs.patch >/tmp/rx-eval-runner-ap-5.out 2>/tmp/rx-eval-runner-ap-5.err; then
    fail "apply_patch rejects absolute paths"
  elif rg -q 'path must be relative' /tmp/rx-eval-runner-ap-5.err; then
    pass "apply_patch rejects absolute paths"
  else
    fail "apply_patch rejects absolute paths"
  fi

  cat > traversal.patch <<'PATCH'
*** Begin Patch
*** Add File: ../x
+bad
*** End Patch
PATCH
  if "$apply_bin" < traversal.patch >/tmp/rx-eval-runner-ap-6.out 2>/tmp/rx-eval-runner-ap-6.err; then
    fail "apply_patch rejects traversal paths"
  elif rg -q "parent path '\\.\\.' is not allowed" /tmp/rx-eval-runner-ap-6.err; then
    pass "apply_patch rejects traversal paths"
  else
    fail "apply_patch rejects traversal paths"
  fi

  cat > mismatch-base.patch <<'PATCH'
*** Begin Patch
*** Add File: b.txt
+one
*** End Patch
PATCH
  cat > mismatch.patch <<'PATCH'
*** Begin Patch
*** Update File: b.txt
@@
-two
+three
*** End Patch
PATCH
  if "$apply_bin" < mismatch-base.patch >/tmp/rx-eval-runner-ap-7a.out 2>/tmp/rx-eval-runner-ap-7a.err \
    && ! "$apply_bin" < mismatch.patch >/tmp/rx-eval-runner-ap-7b.out 2>/tmp/rx-eval-runner-ap-7b.err \
    && [[ "$(cat b.txt)" == "one" ]]; then
    pass "apply_patch mismatched hunk fails without partial write"
  else
    fail "apply_patch mismatched hunk fails without partial write"
  fi

  cat > repeat.patch <<'PATCH'
*** Begin Patch
*** Add File: c.txt
+repeat
*** End Patch
PATCH
  if "$apply_bin" < repeat.patch >/tmp/rx-eval-runner-ap-8a.out 2>/tmp/rx-eval-runner-ap-8a.err \
    && ! "$apply_bin" < repeat.patch >/tmp/rx-eval-runner-ap-8b.out 2>/tmp/rx-eval-runner-ap-8b.err \
    && rg -q "already exists" /tmp/rx-eval-runner-ap-8b.err; then
    pass "apply_patch deterministic repeat behavior"
  else
    fail "apply_patch deterministic repeat behavior"
  fi
fi

if rg -n 'pub struct BashTool' src/tools 2>/dev/null >/dev/null; then
  skip "bash tool eval not implemented in runner"
else
  skip "bash tool eval skipped (tool not present)"
fi

echo
echo "== summary =="
echo "passed: ${#PASSED[@]}"
echo "failed: ${#FAILED[@]}"
echo "skipped: ${#SKIPPED[@]}"
echo "kernel logs:"
echo "  - $kernel_log_done"
echo "  - $kernel_log_cap"
echo "apply_patch temp dir:"
echo "  - $tmp_apply_dir"

if [[ "${#FAILED[@]}" -gt 0 ]]; then
  echo
  echo "failed checks:"
  for check in "${FAILED[@]}"; do
    echo "  - $check"
  done
  exit 1
fi

exit 0
