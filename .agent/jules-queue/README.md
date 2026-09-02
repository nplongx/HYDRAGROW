# Task Queue

Each `TASK-*.md` here is one queued task envelope, dispatched via the
[Juleson](https://github.com/SamyRai/Juleson) CLI (`juleson` / `jsn`):

- `juleson sessions create sources/github/<owner>/<repo> --prompt-file TASK-<id>.md`
  starts one session from an envelope in this directory.
- `juleson sessions batch sources/github/<owner>/<repo> <this-dir> --parallel N`
  dispatches every envelope here as a batch, N at a time.
- `juleson sessions list` / `juleson sessions status` show what's currently running.
- Move a dispatched envelope to `completed/` once its session finishes successfully;
  leave a failed envelope in place for a re-run.
