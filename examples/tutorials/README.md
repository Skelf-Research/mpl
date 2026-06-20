# MPL Tutorials

End-to-end walkthroughs that exercise MPL in real agent workflows. Each
subdirectory is self-contained — its own README explains what it shows,
the prerequisites, and how to run it.

| Tutorial | What it demonstrates |
|---|---|
| [`calendar-workflow/`](calendar-workflow/) | A single agent producing calendar events under `org.calendar.Event.v1` with strict-argcheck QoM. Shows how `start_ts < end_ts` and visibility constraints fail closed in the proxy. |
| [`multi-agent/`](multi-agent/) | A multi-agent pipeline where downstream agents act on upstream payloads through the MPL envelope. Shows semantic-hash continuity across hops and how the policy engine blocks unauthorized side effects. |
| [`rag-workflow/`](rag-workflow/) | A RAG-style answer agent under `eval.rag.Answer.v1` with sources attached. Shows the groundedness metric in action and how the proxy attaches the QoM report to each response. |

## Prerequisites

Each tutorial uses Python ≥ 3.10 and the `mpl-sdk` Python package. From the repo root:

```bash
cd python && uv venv && uv run maturin develop
```

Then `pip install -r requirements.txt` inside the tutorial directory.

You also need the **MPL proxy** and **registry** available — either from `cargo run -p mplx` against `registry/`, or via the demo Docker image (`docker compose up mpl-proxy`).

## Where the contracts come from

Each tutorial's STypes live under `registry/stypes/`. If you change a constraint in CEL or add a new ontology rule, restart the proxy — STypes are loaded at startup. See `docs/registry-architecture.md` for the layout.
