import { spawnSync } from 'node:child_process';
export function tailnetAdvice(config, port) {
  for (const [host, web] of Object.entries(config.Web || {})) {
    const remotePort = host.split(':').at(-1);
    const target = web.Handlers?.['/']?.Proxy;
    if ([`http://127.0.0.1:${port}`, `http://localhost:${port}`].includes(target)
      && config.TCP?.[remotePort]?.HTTPS && !config.AllowFunnel?.[host]) {
      return `Owner URL: https://${host}/?theme=dark&state=seeded`;
    }
  }
  let remotePort = 8515;
  while (config.TCP?.[remotePort]) remotePort++;
  return `No private HTTPS tailnet mapping targets :${port}. Ask the owner to run:\n` +
    `tailscale serve --bg --https=${remotePort} http://127.0.0.1:${port}\n` +
    'Then run node annotate/run.mjs status to print the owner URL. No mapping was changed.';
}
export function printTailnet(port) {
  const result = spawnSync('tailscale', ['serve', 'status', '--json'], { encoding: 'utf8', timeout: 8000 });
  let config = {};
  try { config = JSON.parse(result.stdout); } catch { console.log('Could not read tailscale serve status.'); }
  console.log(tailnetAdvice(config, port));
}
