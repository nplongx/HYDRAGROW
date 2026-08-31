# Task Queue

Each `TASK-*.md` here is one queued task envelope.

- `agentctl task create` writes them
- `agentctl queue` dispatches them
- Dispatched envelopes move to `completed/`; failures stay put for a re-run
