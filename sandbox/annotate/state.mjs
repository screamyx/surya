// Local review history adapted from lavish-live.
import fs from 'node:fs';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
export const defaultStateDir = path.join(import.meta.dirname, '.state');
export function createStore(directory = defaultStateDir) {
  fs.mkdirSync(directory, { recursive: true });
  const file = name => path.join(directory, `${name}.jsonl`);
  const read = name => fs.existsSync(file(name))
    ? fs.readFileSync(file(name), 'utf8').split('\n').filter(Boolean).map(JSON.parse) : [];
  const append = (name, value) => fs.appendFileSync(file(name), JSON.stringify(value) + '\n');
  return {
    list() {
      const replies = read('replies');
      return read('annotations').map(a => {
        const mine = replies.filter(r => r.annotationId === a.id);
        return { ...a, reply: mine.filter(r => r.text).at(-1)?.text ?? null,
          status: mine.at(-1)?.status ?? 'sent' };
      });
    },
    add(payload) {
      const annotation = { ...payload, id: randomUUID(), ts: new Date().toISOString() };
      append('annotations', annotation);
      return annotation;
    },
    answer(id, text, status) {
      const annotation = this.list().find(a => a.id === id);
      if (!annotation) throw new Error('Unknown pin');
      if (status === 'done' && !annotation.reply) throw new Error('Reply before marking a pin done');
      append('replies', { annotationId: id, text: text || null, status, ts: new Date().toISOString() });
    },
    clear() {
      const suffix = new Date().toISOString().replace(/[:.]/g, '-') + '-' + randomUUID();
      for (const name of ['annotations', 'replies']) {
        if (fs.existsSync(file(name))) fs.renameSync(file(name), path.join(directory, `${name}.${suffix}.jsonl`));
      }
    },
  };
}
