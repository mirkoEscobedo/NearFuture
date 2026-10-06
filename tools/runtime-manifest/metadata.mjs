// @ts-check
/** Parse observed game JSON extensions without evaluating source code.
 * @param {string} text @param {string} label */
export function parseGameJson(text, label) {
  try {
    let clean = '';
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (c === '#' || (c === '/' && text[i + 1] === '/')) {
        while (i < text.length && text[i] !== '\n') i++;
        clean += '\n';
      } else if (c === '"' || c === "'") {
        const quote = c;
        let raw = '', escaped = false, closed = false;
        for (i++; i < text.length; i++) {
          const next = text[i];
          if (!escaped && next === quote) { closed = true; break; }
          raw += next;
          if (escaped) escaped = false;
          else if (next === '\\') escaped = true;
        }
        if (!closed) throw new Error('unclosed string');
        const value = quote === '"' ? JSON.parse('"' + raw + '"') :
          JSON.parse('"' + raw.replace(/\\'/g, "'").replace(/"/g, '\\"') + '"');
        clean += JSON.stringify(value);
      } else clean += c;
    }
    let result = '', quoted = false, escaped = false;
    for (let i = 0; i < clean.length; i++) {
      const c = clean[i];
      if (!quoted && c === ',' && /^[\s]*[}\]]/.test(clean.slice(i + 1))) continue;
      result += c;
      if (escaped) escaped = false;
      else if (quoted && c === '\\') escaped = true;
      else if (c === '"') quoted = !quoted;
    }
    return JSON.parse(result);
  } catch { throw new Error(`INVALID_JSON: ${label}; repair the metadata/config JSON and retry.`); }
}

/** Keep numeric/runtime JVM flags; omit arbitrary properties, paths, agents and arguments.
 * @param {string} text */
export function safeJvmFlags(text) {
  return (text.match(/"[^"]*"|\S+/g) ?? []).filter(token =>
    /^-X(?:mx|ms|ss)\d+[kKmMgG]$/.test(token) ||
    /^-XX:[+-][A-Za-z0-9]+$/.test(token) ||
    /^-XX:[A-Za-z0-9]+=-?[0-9]+(?:\.[0-9]+)?[kKmMgG]?$/.test(token) ||
    /^-XX:ShenandoahGCMode=(?:satb|iu|generational)$/.test(token) ||
    /^-XX:ShenandoahGCHeuristics=(?:adaptive|static|compact|aggressive)$/.test(token) ||
    /^--(?:enable-preview|add-opens=[A-Za-z0-9./]+=[A-Za-z0-9,-]+|add-exports=[A-Za-z0-9./]+=[A-Za-z0-9,-]+)$/.test(token) ||
    /^-Xlog:async$/.test(token) || token === '-noverify');
}
