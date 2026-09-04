# Probe: does a CopilotKit / AG-UI run survive the browser disconnecting?

Date: 2026-09-05.
Probe dir (kept): `/store/tmp-agui-probe`.
Versions installed and measured: `@copilotkit/runtime` 1.69.2, `@ag-ui/core` 0.0.58, `@ag-ui/client` 0.0.57, `@ag-ui/claude-agent-sdk` 0.0.3, `@anthropic-ai/claude-agent-sdk` 0.3.246, rxjs 7.8.2, Node 24.18.0.

## Answer

The server-side run keeps going: when the client disconnects the runtime only unsubscribes the HTTP reader from a hot ReplaySubject, while the agent itself runs on to RUN_FINISHED in a fire-and-forget async function, so nothing is aborted and every event after the disconnect is lost rather than cancelled.

## Runs

All runs used `POST {basePath}/agent/{agentId}/run` with an `@ag-ui/core` `RunAgentInputSchema` body.
The fake agent emits RUN_STARTED, TEXT_MESSAGE_START, 40 TEXT_MESSAGE_CONTENT ticks one per second, TEXT_MESSAGE_END, RUN_FINISHED - 44 events in total.
"Teardown" means the rxjs teardown function returned from the agent's own `run()` observable.

| Scenario | Emitted (server) | Delivered to client | Logged after disconnect | Teardown fired at | Verdict |
|---|---|---|---|---|---|
| A. Fake agent, curl killed at 5s | emitted=44 ticks=40 | client_events=6 client_ticks=4 | logged_after_disconnect=38 | t+35.0s, at natural completion only | Run survived, reached RUN_FINISHED |
| B. Fake agent, client connected throughout (control) | emitted=44 ticks=40 | client_events=44 client_ticks=40 | n/a | t+40.0s, at natural completion | Instrument known good, 40 of 40 delivered |
| C. Fake agent, explicit `POST .../stop/{threadId}` at 5s | emitted=44 ticks=40 | n/a | n/a | t+40.0s, at natural completion | Stop returned `{"stopped":true}` and `abortRun()` fired at exactly t+5.0s, but the run still finished all 40 ticks |
| D. Real `ClaudeAgentAdapter`, curl killed at 5s | server_events=29 | client_events=7 | 22 events after the disconnect | t+34.6s, at natural completion only | Run survived; `claude` child alive 4s after disconnect (claude_procs=1 of 1), gone after finish (claude_procs=0), RUN_FINISHED reached |

Notes on the counters.

Run A: `http_res_close` logged at t_ms=17268, RUN_FINISHED at t_ms=52314.
`abort_signal_fired=0 abort_run_called=0` - the agent's `run()` is handed no AbortSignal at all (`signal_present=false`), so there is nothing for the agent to observe.

Run B is the control that proves the instrument counts real positives: connected the whole time, the client received 40 of 40 ticks and the RUN_FINISHED event, HTTP 200, 40.0s wall clock.

Run C is the second control, and it proves the teardown/abort instrument can fire early at all: `ABORT_RUN_CALLED` logged at t+5.000s.
It fired and the run continued anyway.
So run A's `abort_run_called=0` is a real absence, not a dead counter.

Run D was repeated three times (D1, D2, D3) with the same shape.
D3 numbers are the ones in the table.
D1: disconnect at t_ms=14180, RUN_FINISHED at t_ms=47505, that is 33.3s after the disconnect.
D2: disconnect at t_ms=68161, RUN_FINISHED at t_ms=102390, 34.2s after, server_events=31 vs client_events=7.
D3: disconnect at t_ms=120822, RUN_FINISHED at t_ms=155391, 34.6s after.
In D3 the `claude` process (`/home/user2/.local/state/claude-wrapper/claude`, a direct child of the server process) was still running 4 seconds after the disconnect, with its own `bash` child running the sleep loop.

## The source that explains it

### 1. The Node adapter really does abort the request signal on socket close

`/store/tmp-agui-probe/node_modules/@remix-run/node-fetch-server/dist/lib/request-listener.js`, lines 169-176:

```js
export function createRequest(req, res, options) {
    let controller = new AbortController();
    // Abort once we can no longer write a response if we have
    // not yet sent a response (i.e., `close` without `finish`)
    // `finish` -> done rendering the response
    // `close` -> response can no longer be written to
    res.once('close', () => controller?.abort());
    res.once('finish', () => (controller = null));
```

So `request.signal` is not inert.
The disconnect is detected.

### 2. The runtime reacts to the abort by unsubscribing the SSE reader, and nothing else

`/store/tmp-agui-probe/node_modules/@copilotkit/runtime/dist/v2/runtime/handlers/shared/sse-response.mjs`, lines 118-120:

```js
	request.signal.addEventListener("abort", () => {
		subscription?.unsubscribe();
	});
```

Line 112, the same idea for a request that was already dead when the observable resolved:

```js
		if (request.signal.aborted) subscription.unsubscribe();
```

Line 77, the write guard, which is what actually drops the post-disconnect events:

```js
				if (!request.signal.aborted && !streamClosed) try {
```

There is no call to `runner.stop`, no `agent.abortRun`, no cancellation of anything upstream.

### 3. What is being unsubscribed is a hot ReplaySubject, not the agent

`/store/tmp-agui-probe/node_modules/@copilotkit/runtime/dist/v2/runtime/runner/in-memory.mjs`.

Line 332 opens a plain async function:

```js
		const runAgent = async () => {
```

Lines 371-372, inside it, drive the agent and push each event into the subjects:

```js
			try {
				await request.agent.runAgent(request.input, {
```

Lines 404-405, the key two lines:

```js
		runAgent();
		return runSubject.asObservable();
```

`runAgent()` is called without `await` and its promise is discarded.
The returned observable is a `ReplaySubject` view, created at line 330.
Unsubscribing from a ReplaySubject detaches one reader.
It cannot reach back and stop the async function that is feeding it.

### 4. Even the explicit stop endpoint is only a request, not a kill

`/store/tmp-agui-probe/node_modules/@copilotkit/runtime/dist/v2/runtime/handlers/handle-stop.mjs`, line 15:

```js
		if (!await runtime.runner.stop({ threadId })) return new Response(JSON.stringify({
```

`/store/tmp-agui-probe/node_modules/@copilotkit/runtime/dist/v2/runtime/runner/in-memory.mjs`, line 454:

```js
			agent.abortRun();
```

And `abortRun` on the base class is empty.
`/store/tmp-agui-probe/node_modules/@ag-ui/client/dist/index.mjs` (minified, one line, byte offset 38206):

```js
abortRun(){}
```

Only `HttpAgent` overrides it, and only to abort its own outbound fetch:

```js
abortRun(){this.abortController.abort(),super.abortRun()}
```

### 5. The Claude adapter implements neither teardown nor abortRun

`/store/tmp-agui-probe/node_modules/@ag-ui/claude-agent-sdk/dist/index.mjs`, lines 416-417 and 455-462:

```js
  run(input) {
    return new Observable((subscriber) => {
      ...
      this.activeQueries.set(threadId, queryStream);
      this.translateStream(runInput, queryStream, subscriber).catch((error) => {
        subscriber.error(error);
      }).finally(() => {
        if (timeoutHandle !== void 0) clearTimeout(timeoutHandle);
        this.activeQueries.delete(threadId);
      });
    });
```

The observable's subscribe function returns nothing.
There is no teardown, so unsubscribing is a no-op by construction.
`grep -c abortRun` over that file returns 0.
There is an `interrupt()` at line 411 that does reach the query stream, but the runtime never calls it - the stop path calls `abortRun`, not `interrupt`.

An `AbortController` is only created when `queryTimeoutMs` is configured, and it is wired to a timeout, not to the request.

## What this means for the daemon

Runs are already decoupled from the HTTP request in this stack.
That is the good news and the trap at the same time.

If the daemon keeps the PR 568 wiring as-is:

1. A closed browser does not stop the work.
   The `claude` child keeps running and the run reaches RUN_FINISHED.
2. But every event produced after the disconnect is thrown away at `sse-response.mjs` line 77.
   The run finishes and its output goes nowhere the user can see.
3. Nothing kills a run.
   Not a disconnect, and not the `/stop` endpoint either, because `ClaudeAgentAdapter` has no `abortRun`.
   A wedged run is a leaked `claude` process until the daemon is restarted.

So the daemon must do three things.

**Replay, not just survive.**
The `InMemoryAgentRunner` already keeps a per-thread `ReplaySubject` and a historic-run store, and it exposes `POST {basePath}/agent/{agentId}/connect`.
Reconnect must go through `connect`, not a fresh `run`, or the browser re-triggers the work instead of rejoining it.
Verify the replay window is unbounded enough for a long run; `ReplaySubject(Infinity)` is what the code constructs, but the shared store is process-global and bounded by `ɵnormalizeLimits`, so a long run can be trimmed.

**Own the process lifecycle.**
Do not rely on `/stop`.
Either subclass `ClaudeAgentAdapter` to implement `abortRun()` by calling its existing `interrupt()`, or track the spawned `claude` pid yourself and kill it.
Without one of these, "cancel" is a lie and a stuck run is unkillable.

**Do not rely on process memory for durability.**
The in-memory runner is per-process.
A daemon restart loses every in-flight run and its replay buffer.
If runs must survive a deploy, the run state has to live outside the Node process, which means a custom `AgentRunner` rather than `InMemoryAgentRunner`.

One more caveat worth pinning.
`InMemoryAgentRunner.run` throws `Thread already running` on a second concurrent run for the same `threadId` unless `onConcurrentRun` is `"supersede"`, and supersede calls `priorAgent.abortRun()` - which, for the Claude adapter, does nothing.
So a reconnect that accidentally re-runs a busy thread gets two live `claude` processes on one thread, not one.
