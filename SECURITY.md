# Security policy

## Reporting a vulnerability

Report privately through GitHub, at https://github.com/screamyx/surya/security/advisories/new.
Do not open a public issue for a security problem.

If that page is not available, open an issue that says only that you have a security report and asks for a private channel.
Put no detail in it.

Say what you did, what happened, and what you expected.
A proof of concept helps.
Expect a first reply within a week.

surya is pre-1.0 and has no release stream yet.
The only supported version is the newest commit on `main`.

## What is in scope

- The engine's WebSocket RPC socket, including the token check on a non-loopback bind.
- The app's connection to an engine, and how it stores the token it dialed with.
- The permission flow: anything that lets an agent run a tool the user did not allow.
- The browser pane's CDP bridge, where an agent drives a page.
- The installers under `deploy/`.

## What is out of scope

surya runs Claude Code on your own machine, as you.
An agent can therefore read and write every file your account can, run commands, and reach your network.
That is what the app is for, so it is not a vulnerability by itself.

Anything an agent does after you approve it is out of scope.
So is Claude Code itself, which is a separate product.

## The engine socket is trusted by default

The engine serves its RPC on `127.0.0.1:27700` by default, with no token.
`crates/engine/src/ipc.rs:3` states it plainly: "Loopback stays exactly as it always was: `127.0.0.1:<port>`, no token."

So any process running as any user who can reach that loopback port can drive the engine.
It can start agents, read files through the engine, and take over the browser pane.
Treat the machine you run the engine on as trusted, and do not run it on a shared box you do not control.

Binding anywhere else is a different setting.
`crates/engine/src/ipc.rs:4-8` says binding to a LAN, VPN or tailnet address, or to `0.0.0.0`, "requires a shared token", read from `SURYA_IPC_TOKEN` or from `{data_dir}/ipc-token`, which is generated on the first non-loopback start and kept at mode 0600.
A remote app presents it as `Authorization: Bearer <token>`.

There is no TLS on that socket, on purpose.
`crates/engine/src/ipc.rs:10-11` gives the reason: the networks it targets, "tailnet, VPN, LAN", carry their own encryption.
Do not expose the port to the open internet.

## Known by design

`browser_eval` runs whatever JavaScript an agent hands it, in the browser pane, with no restriction.
That is the tool's purpose.
An agent that reaches the engine can therefore run script in any page the pane has open, including a signed-in one.

## Please do not

- Test against machines or accounts that are not yours.
- Run denial of service against anything.
- Report findings that need physical access to an unlocked machine.
