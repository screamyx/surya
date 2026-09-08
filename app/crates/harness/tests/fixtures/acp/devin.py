#!/usr/bin/env python3
import json, pathlib, sys, threading, time
root = pathlib.Path(__file__).parent

def emit(frame):
    print(json.dumps(dict(jsonrpc='2.0', **frame)), flush=True)

def config(model):
    return [{'id':'model', 'category':'model', 'type':'select',
             'currentValue':'gpt-old', 'options':[{'value':model, 'name':model}]}]

if sys.argv[1:] == ['models', 'list', '--format', 'json']:
    with (root / 'probes').open('a') as f: f.write('probe\n')
    state = (root / 'state').read_text()
    if state == 'hang': time.sleep(60)
    if state == 'error':
        print('account unavailable', file=sys.stderr)
        sys.exit(1)
    time.sleep(0.1)
    print(json.dumps({'families':[{'variants':[{'model_uid':state, 'label':state}]}]}))
    sys.exit(0)
assert sys.argv[1:] == ['acp'], sys.argv
selected = None

def refresh():
    # An unrelated session must not satisfy the requested-model wait.
    emit({'method':'session/update', 'params':{'sessionId':'other', 'update':{
        'sessionUpdate':'config_option_update', 'configOptions':config('gpt-new')}}})
    time.sleep(0.1)
    (root / 'refreshed').touch()
    emit({'method':'session/update', 'params':{'sessionId':'s-1', 'update':{
        'sessionUpdate':'config_option_update', 'configOptions':config('gpt-new')}}})

for line in sys.stdin:
    req = json.loads(line)
    method = req.get('method')
    result = {}
    if method == 'initialize': result = {'protocolVersion':1, 'agentCapabilities':{}}
    elif method == 'session/new':
        result = {'sessionId':'s-1', 'configOptions':config('gpt-old')}
    elif method == 'session/set_config_option':
        selected = req['params']['value']
        assert (root / 'refreshed').exists(), 'selected before own session refreshed'
        if (root / 'state').read_text() == 'reject':
            emit({'id':req['id'], 'error':{'code':-32602, 'message':'model unavailable'}})
            continue
        assert selected == 'gpt-new', selected
    elif method == 'session/prompt':
        (root / 'prompted').write_text(selected or 'default')
        result = {'stopReason':'end_turn'}
    early = (root / 'state').read_text() == 'early'
    if method == 'session/new' and early: refresh()
    emit({'id':req['id'], 'result':result})
    if method == 'session/new' and not early: threading.Thread(target=refresh, daemon=True).start()
    if method == 'session/prompt': break
