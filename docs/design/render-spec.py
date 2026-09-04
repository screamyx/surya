import re, json, pathlib
src = pathlib.Path('/store/agent-worktrees/surya-theme/app/crates/theme/src/builtins.rs').read_text()

def seeds(fn):
    body = src.split(f'fn {fn}() -> ThemeVariant {{')[1].split('\n}')[0]
    out = {}
    for k in ['background','shell','raised','card','text','muted','faint','accent','danger','warning','success','terminal_background']:
        m = re.search(rf'^\s*{k}: "(#[0-9a-fA-F]+)",', body, re.M)
        out[k] = m.group(1)
    return out

light, dark = seeds('surya_light'), seeds('surya_dark')

surya = pathlib.Path('/store/agent-worktrees/surya-theme/app/crates/ui/src/surya.rs').read_text()
def const(name):
    m = re.search(rf'pub const {name}: f32 = ([0-9.]+)', surya)
    return float(m.group(1))
G = {n: const(n) for n in ['CANVAS_INSET','PANEL_GAP','PANEL_RADIUS','FLOAT_RADIUS','PANEL_PAD','COMPOSER_RESTING_HEIGHT']}
G['CONTROL_RADIUS'] = 6.0
G['COMPOSER_RADIUS'] = G['COMPOSER_RESTING_HEIGHT']/2

def hexa(h, a):
    h=h.lstrip('#'); r,g,b=(int(h[i:i+2],16) for i in (0,2,4))
    return f'rgba({r},{g},{b},{a})'

def scene(t, dark_mode):
    border = hexa('#ffffff' if dark_mode else '#000000', 0.10 if dark_mode else 0.12)
    shadow = '0 1px 14px rgba(20,17,5,0.24)' if dark_mode else '0 1px 10px rgba(20,17,5,0.06)'
    fshadow = '0 4px 22px rgba(20,17,5,0.34)' if dark_mode else '0 4px 16px rgba(20,17,5,0.10)'
    return f'''
<div class="win" style="background:{t['shell']};padding:{G['CANVAS_INSET']}px;color:{t['text']}">
  <div class="rail" style="background:{t['background']};border:1px solid {border};border-radius:{G['PANEL_RADIUS']}px;box-shadow:{shadow};margin-right:{G['PANEL_GAP']}px">
    <div class="railpad">
      <div class="eyebrow" style="color:{t['faint']}">Needs you</div>
      <div class="row sel" style="background:{hexa(t['accent'],0.12)}"><span class="dot" style="background:{t['accent']}"></span><span>Fix the duty rounding</span><span class="num" style="color:{t['muted']}">3</span></div>
      <div class="row"><span class="dot" style="background:{t['warning']}"></span><span>Approve migration</span><span class="num" style="color:{t['muted']}">1</span></div>
      <div class="eyebrow" style="color:{t['faint']};margin-top:18px">Running</div>
      <div class="row"><span class="dot" style="background:{t['success']}"></span><span>catalog-sync backfill</span><span class="num" style="color:{t['muted']}">12</span></div>
      <div class="row"><span class="dot" style="background:{t['success']}"></span><span>Rename the invoice column</span><span class="num" style="color:{t['muted']}">7</span></div>
      <div class="eyebrow" style="color:{t['faint']};margin-top:18px">Done, unshipped</div>
      <div class="row"><span class="dot" style="background:{t['muted']}"></span><span>Photo cover constraint</span><span class="num" style="color:{t['muted']}">40</span></div>
    </div>
  </div>
  <div class="main" style="background:{t['background']};border:1px solid {border};border-radius:{G['PANEL_RADIUS']}px;box-shadow:{shadow};margin-right:{G['PANEL_GAP']}px">
    <div class="mainpad">
      <div class="title">Fix the duty rounding</div>
      <div class="path" style="color:{t['muted']}">backend/app/Services/Duty/CifValue.php</div>
      <p class="body">The CIF value rounds at two decimals before the duty rate applies, so a car landing at 84,999.995 pays duty on 85,000. Three units this month sit on the wrong side of it.</p>
      <p class="body">I changed the order: rate first, round last. The four tests that pinned the old figures now pin the new ones.</p>
      <div class="diffline" style="background:{hexa(t['success'],0.10)};color:{t['success']}">+ return round($base * $rate, 2);</div>
      <div class="diffline" style="background:{hexa(t['danger'],0.10)};color:{t['danger']}">- return round($base, 2) * $rate;</div>
      <div class="caption" style="color:{t['faint']}">4 files changed &middot; 06:14</div>
    </div>
    <div class="pill" style="background:{t['card']};border:1px solid {border};border-radius:{G['COMPOSER_RADIUS']}px;box-shadow:{fshadow};height:{G['COMPOSER_RESTING_HEIGHT']}px">
      <span style="color:{t['faint']}">Ask anything</span>
      <span class="send" style="background:{t['accent']}"></span>
    </div>
  </div>
  <div class="right" style="background:{t['background']};border:1px solid {border};border-radius:{G['PANEL_RADIUS']}px;box-shadow:{shadow}">
    <div class="urlbar" style="border-bottom:1px solid {border}">
      <span class="chip" style="background:{t['raised']};border-radius:{G['CONTROL_RADIUS']}px;color:{t['muted']}">Browser</span>
      <span class="url" style="color:{t['muted']}">localhost:5173/vehicles</span>
    </div>
    <div class="files" style="background:{t['card']};border:1px solid {border};border-radius:{G['FLOAT_RADIUS']}px;box-shadow:{fshadow}">
      <div class="frow" style="color:{t['muted']}">app/</div>
      <div class="frow" style="color:{t['text']}">&nbsp;&nbsp;CifValue.php</div>
      <div class="frow" style="color:{t['muted']}">&nbsp;&nbsp;DutyTable.php</div>
      <div class="frow" style="color:{t['muted']}">tests/</div>
    </div>
  </div>
</div>'''

css = f'''
@font-face {{ font-family: x; src: local("Inter"), local("DejaVu Sans"); }}
* {{ box-sizing: border-box; margin:0; }}
body {{ font-family: Inter, "DejaVu Sans", system-ui, sans-serif; -webkit-font-smoothing: antialiased; }}
.win {{ width:1440px; height:900px; display:flex; }}
.rail {{ width:264px; flex:none; overflow:hidden; }}
.railpad {{ padding:{G['PANEL_PAD']}px; padding-top:38px; }}
.main {{ flex:1; min-width:0; position:relative; overflow:hidden; display:flex; flex-direction:column; }}
.mainpad {{ padding:38px 40px 0 40px; flex:1; }}
.right {{ width:420px; flex:none; overflow:hidden; position:relative; padding-top:38px; }}
.eyebrow {{ font-size:11px; font-weight:500; padding:0 8px 6px; }}
.row {{ display:flex; align-items:center; gap:8px; font-size:13px; font-weight:500; height:30px; padding:0 8px; border-radius:{G['CONTROL_RADIUS']}px; }}
.row .num {{ margin-left:auto; font-size:11px; font-variant-numeric: tabular-nums; }}
.dot {{ width:6px; height:6px; border-radius:50%; flex:none; }}
.title {{ font-size:28px; line-height:32px; font-weight:600; letter-spacing:-0.3px; }}
.path {{ font-family: "DejaVu Sans Mono", monospace; font-size:12.5px; line-height:18px; margin-top:6px; }}
.body {{ font-size:14px; line-height:21px; margin-top:16px; max-width:65ch; }}
.diffline {{ font-family:"DejaVu Sans Mono", monospace; font-size:12.5px; line-height:20px; padding:0 8px; margin-top:10px; border-radius:4px; }}
.caption {{ font-size:11px; font-weight:500; margin-top:18px; }}
.pill {{ position:absolute; left:40px; right:40px; bottom:16px; display:flex; align-items:center; padding:0 8px 0 18px; font-size:14px; }}
.send {{ margin-left:auto; width:30px; height:30px; border-radius:50%; }}
.urlbar {{ display:flex; align-items:center; gap:8px; padding:0 10px 10px; font-size:12px; }}
.chip {{ padding:4px 8px; font-size:11px; font-weight:500; }}
.url {{ font-family:"DejaVu Sans Mono", monospace; font-size:12.5px; }}
.files {{ position:absolute; left:16px; right:16px; top:120px; padding:8px; font-size:12.5px; font-family:"DejaVu Sans Mono", monospace; }}
.frow {{ line-height:22px; padding:0 6px; }}
'''

for name, t, d in [('light', light, False), ('dark', dark, True)]:
    html = f'<!doctype html><meta charset=utf-8><style>{css}</style>{scene(t, d)}'
    pathlib.Path(f'/tmp/surya-spec-{name}.html').write_text(html)
print(json.dumps({'light':light,'dark':dark,'geom':G}, indent=1))
