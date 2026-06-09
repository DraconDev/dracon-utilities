Fix the real bugs and will-bite-eventually issues found in the dracon-voice-notifications full review.

## Goals
1. Fix lock file TOCTOU + PID write bug (daemon.rs:66-82)
2. Fix git diff subprocess timeout (daemon.rs:455)
3. Add monitor restart logic (monitor.rs, journal.rs)
4. Fix Semaphore::acquire().expect() panic sites (6 sites in daemon.rs)
5. Fix set_var thread safety (tts.rs:27-29)
6. Build + test after each change
7. Commit with clear messages

## Checklist
- [ ] Lock file: use create_new directly, never File::create for PID write
- [ ] Git diff: add timeout via tokio::process or std::process timeout wrapper
- [ ] Monitor restart: wrap run() in retry loop with backoff
- [ ] Journal restart: same pattern
- [ ] Semaphore: replace .expect("Semaphore closed") with log-and-skip
- [ ] set_var: move before runtime or guard with OnceLock
- [ ] Build passes (0 errors, 0 warnings)
- [ ] 29 unit tests pass
