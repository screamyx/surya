#!/usr/bin/env python3
"""Drive `claude -p --input-format stream-json` for one scenario and print what matters.
Usage: driver.py <scenario> ; scenarios: model, ask, resume1, resume2 <session_id>, interrupt
Every counter is printed as a pair so a zero shows what it counted out of."""
import json, os, subprocess, sys, threading, time, queue

ENV = {k: v for k, v in os.environ.items() if k not in ("CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "AGB_AGENT_ID", "AGB_AGENT_RECAP", "AGB_RUNTIME")}
if os.environ.get("SDK_ENTRY"): ENV["CLAUDE_CODE_ENTRYPOINT"] = os.environ["SDK_ENTRY"]
BASE = [os.environ.get("CLAUDE_BIN", "claude"), "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
        "--permission-prompts", "host", "--permission-prompt-tool", "stdio", "--model", "claude-haiku-4-5-20251001"]

def user(text):
    return json.dumps({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}) + "\n"

def start(extra):
    p = subprocess.Popen(BASE + extra, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=ENV, cwd="/tmp/surya-probes")
    q = queue.Queue()
    def pump():
        for line in p.stdout:
            q.put(line)
        q.put(None)
    threading.Thread(target=pump, daemon=True).start()
    return p, q

def read_until(q, pred, timeout):
    """Collect events until pred(event) is true or timeout. Returns (events, hit)."""
    out, t0 = [], time.time()
    while time.time() - t0 < timeout:
        try:
            line = q.get(timeout=0.5)
        except queue.Empty:
            continue
        if line is None:
            return out, False
        try:
            ev = json.loads(line)
        except Exception:
            continue
        out.append(ev)
        if pred(ev):
            return out, True
    return out, False

def is_result(ev): return ev.get("type") == "result"
def summarize(evs):
    for ev in evs:
        t = ev.get("type")
        if t == "assistant":
            m = ev["message"]
            for c in m.get("content", []):
                if c.get("type") == "text": print(f"  assistant[{m.get('model')}]: {c['text'][:200]!r}")
                elif c.get("type") == "tool_use": print(f"  assistant tool_use: {c.get('name')} input={json.dumps(c.get('input'))[:200]}")
        elif t == "result": print(f"  result: subtype={ev.get('subtype')} turns={ev.get('num_turns')} text={str(ev.get('result',''))[:160]!r}")
        elif t == "control_request": print(f"  CONTROL_REQUEST: {json.dumps(ev)[:400]}")
        elif t == "system" and ev.get("subtype") == "init": print(f"  init: session={ev.get('session_id')} model={ev.get('model')}")
        elif t == "user":
            for c in ev["message"].get("content", []):
                if isinstance(c, dict) and c.get("type") == "tool_result": print(f"  tool_result: {str(c.get('content'))[:160]!r}")
        else:
            print(f"  {t}/{ev.get('subtype','')}: {json.dumps(ev)[:160]}")

sc = sys.argv[1]
if sc == "tools":
    p, q = start([])
    p.stdin.write(json.dumps({"type": "control_request", "request_id": "init-1", "request": {"subtype": "initialize", "hooks": {}, "supportedDialogKinds": ["refusal_fallback_prompt"]}}) + "\n"); p.stdin.flush()
    p.stdin.write(user("say ok")); p.stdin.flush()
    evs, hit = read_until(q, is_result, 60)
    init = next((e for e in evs if e.get("type") == "system" and e.get("subtype") == "init"), {})
    t = init.get("tools", [])
    print(f"open-input tools={len(t)} AskUserQuestion={'AskUserQuestion' in t} EnterPlanMode={'EnterPlanMode' in t}")
    p.stdin.close(); p.wait(timeout=10)
    sys.exit(0)
if sc == "model":
    p, q = start([])
    p.stdin.write(user("/model sonnet")); p.stdin.flush()
    evs, hit = read_until(q, is_result, 60); print("turn1 (/model sonnet):"); summarize(evs)
    p.stdin.write(user("Reply with exactly one word: pong")); p.stdin.flush()
    evs, hit = read_until(q, is_result, 90); print("turn2 (after switch):"); summarize(evs)
    models = [ev["message"].get("model") for ev in evs if ev.get("type") == "assistant"]
    print(f"asked=1 switched={'1' if any(m and 'sonnet' in m for m in models) else '0'} models_seen={models}")
    p.stdin.close(); p.wait(timeout=10)
elif sc == "ask":
    p, q = start([])
    p.stdin.write(json.dumps({"type": "control_request", "request_id": "init-1", "request": {"subtype": "initialize", "hooks": {}}}) + "\n"); p.stdin.flush()
    evs, hit = read_until(q, lambda e: e.get("type") == "control_response", 30); print("initialize handshake:"); summarize(evs)
    p.stdin.write(user("Use the AskUserQuestion tool to ask me one question: do I prefer option A or option B? Offer both as options. Then wait for my answer and reply with the letter I chose.")); p.stdin.flush()
    evs, hit = read_until(q, lambda e: e.get("type") == "control_request" or is_result(e), 90)
    print("after asking:"); summarize(evs)
    cr = [e for e in evs if e.get("type") == "control_request"]
    print(f"asked=1 control_requests={len(cr)} result_seen={'1' if any(is_result(e) for e in evs) else '0'}")
    if cr:
        req = cr[0]; rid = req.get("request_id"); sub = req.get("request", {}).get("subtype")
        print(f"  request subtype={sub} keys={list(req.get('request',{}).keys())}")
        # answer: allow the tool, with the chosen option as updated input if the request carries the tool input
        inp = req.get("request", {}).get("input", {})
        print("  tool_name=", req.get("request", {}).get("tool_name"), " input=", json.dumps(inp)[:500])
        upd = dict(inp)
        qs = inp.get("questions") or []
        if qs:
            upd["answers"] = {qs[0].get("question", "q"): (qs[0].get("options") or [{}])[0].get("label", "A")}
        answer = {"type": "control_response", "response": {"subtype": "success", "request_id": rid, "response": {"behavior": "allow", "updatedInput": upd}}}
        print("  answering with:", json.dumps(upd.get("answers")))
        p.stdin.write(json.dumps(answer) + "\n"); p.stdin.flush()
        evs, hit = read_until(q, is_result, 90); print("after control_response:"); summarize(evs)
    p.stdin.close(); p.wait(timeout=10)
elif sc == "resume1":
    p, q = start([])
    p.stdin.write(user("Remember the secret word: pineapple. Reply with just 'stored'.")); p.stdin.flush()
    evs, hit = read_until(q, is_result, 60); summarize(evs)
    sid = next((e.get("session_id") for e in evs if e.get("type") == "system"), None) or next((e.get("session_id") for e in evs if e.get("session_id")), None)
    print(f"SESSION={sid}")
    p.stdin.close(); p.wait(timeout=10)
elif sc == "resume2":
    sid = sys.argv[2]
    p, q = start(["--resume", sid])
    p.stdin.write(user("What was the secret word? Reply with just the word.")); p.stdin.flush()
    evs, hit = read_until(q, is_result, 60); summarize(evs)
    txt = " ".join(c["text"] for e in evs if e.get("type") == "assistant" for c in e["message"]["content"] if c.get("type") == "text")
    print(f"asked=1 remembered={'1' if 'pineapple' in txt.lower() else '0'}")
    p.stdin.close(); p.wait(timeout=10)
elif sc == "interrupt":
    p, q = start(["--permission-mode", "bypassPermissions", "--dangerously-skip-permissions", "--allowedTools", "Bash"])
    t0 = time.time()
    p.stdin.write(user("Run this exact Bash command in the foreground (never background) and wait for it: until [ -f /tmp/surya-probes/stop-flag ]; do sleep 2; done; echo LOOP_DONE . Then reply with its output.")); p.stdin.flush()
    evs, hit = read_until(q, lambda e: e.get("type") == "assistant" and any(c.get("type") == "tool_use" for c in e["message"]["content"]), 60)
    print("tool call seen:", hit); summarize(evs)
    time.sleep(3)
    sleeps_before = subprocess.run(["pgrep", "-fc", "stop-flag"], capture_output=True, text=True).stdout.strip()
    p.stdin.write(json.dumps({"type": "control_request", "request_id": "int-1", "request": {"subtype": "interrupt"}}) + "\n"); p.stdin.flush()
    t_int = time.time()
    evs, hit = read_until(q, is_result, 60); print("after interrupt:"); summarize(evs)
    time.sleep(2)
    sleeps_after = subprocess.run(["pgrep", "-fc", "stop-flag"], capture_output=True, text=True).stdout.strip()
    print(f"asked=1 result_after_interrupt={'1' if hit else '0'} seconds_from_interrupt_to_result={round(time.time()-t_int,1)} total={round(time.time()-t0,1)} loop_before={sleeps_before} loop_after={sleeps_after}")
    p.stdin.close(); p.wait(timeout=10)
