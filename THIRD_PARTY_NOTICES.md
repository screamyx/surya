# Third-party notices

This file covers third-party material vendored into the repository as source.
Notices for code the built application bundles live in `app/THIRD_PARTY_NOTICES.md`.
That file opens with "Surya bundles the following syntax-highlighting components", so its scope is the shipped binary.
The design kit below ships in no binary, which is why it is recorded here instead.

surya's own licence is the root `LICENSE` (MIT).
The `app/` subtree keeps its upstream licence in `app/LICENSE` (MIT, Copyright (c) 2026 Wing), because `app/` is zeronsh/comet imported as a git subtree (decision 22).

## ux-ui-agent-skills (the design kit)

| Field | Value |
| --- | --- |
| Package | `ux-ui-agent-skills` |
| Version | 2.5.1 |
| Author | plugin87 |
| Licence | MIT (declared) |
| Upstream | https://github.com/plugin87/ux-ui-agent-skills |
| Registry | https://registry.npmjs.org/ux-ui-agent-skills/-/ux-ui-agent-skills-2.5.1.tgz |

Vendored into `tokens/`, `taste/`, `components/`, `accessibility/`, `frameworks/`, `workflows/`, `content/`, `design-systems/` and `scripts/`.
Of the 234 kit files under those directories, 214 are byte-identical to the 2.5.1 tarball and 20 carry local edits.
The tarball was checked against the registry's published `dist.shasum`, `8c9fc36e3c83f8c19f7c3949c4384977865e01cb`.

### What upstream states, and what it does not

The 2.5.1 tarball's `package/package.json` carries `"license": "MIT"`.
The tarball's `package/README.md` line 598 reads: "Released under the **[MIT License](https://opensource.org/licenses/MIT)**."

Upstream ships no `LICENSE` file and no copyright line.
`https://api.github.com/repos/plugin87/ux-ui-agent-skills` returns `"license": null`, and `LICENSE`, `LICENSE.md` and `LICENSE.txt` all return HTTP 404 from `https://raw.githubusercontent.com/plugin87/ux-ui-agent-skills/main/`.
No file matching `license`, `notice` or `copying` exists anywhere in the 2.5.1 tarball.

So there is no upstream copyright notice to reproduce.
The MIT terms that the two declarations above select are reproduced here in full, with the copyright line left as upstream states it, which is only the author name.

```
MIT License

Copyright (c) plugin87

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

Open item for the owner: ask plugin87 to add a `LICENSE` file with a real copyright line, then replace the block above with the exact upstream text.
