// This source executes only in the generated CJS alias, never in application output.
export function aliasSource(names) {
  return `'use strict'; // W72_REAL_ALIAS: restore from the adjacent run backup after a forced kill.
const fs = require('node:fs'), crypto = require('node:crypto');
const { threadId } = require('node:worker_threads');
const initStart = process.hrtime.bigint();
const binding = require(process.env.W72_BINDING);
const initializationMs = Number(process.hrtime.bigint() - initStart) / 1e6;
const started = process.hrtime.bigint(), metrics = {}, observations = [], states = {}, observationErrors = [];
const token = crypto.randomUUID();
let depth = 0, insideNs = 0n, observationNs = 0n;
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
const encode = value => JSON.stringify(value, (_, item) => item instanceof Map ? { mapEntries: [...item] } : item instanceof Set ? { setValues: [...item] } : item);
function observe(name, args, value) {
  const start = process.hrtime.bigint();
  try {
    if (name === 'codeExtract' || name === 'codeExtractWithoutSourceMap') {
      const record = { api: name, filename: args[0], inputSha256: hash(args[1]) };
      for (const key of ['code', 'css', 'cssFile', 'map', 'dependencies', 'updatedBaseStyle']) {
        const field = value[key];
        record[key] = field == null ? null : hash(typeof field === 'string' ? field : encode(field));
      }
      observations.push(record);
    } else if (/^export/.test(name)) states[name] = hash(encode(value));
  } catch (error) { observationErrors.push({ name, error: String(error) }); }
  finally { observationNs += process.hrtime.bigint() - start; }
}
function wrap(name) {
  const fn = binding[name];
  if (typeof fn !== 'function') return fn;
  function timed(invoke, args) {
    const outer = depth++ === 0, start = process.hrtime.bigint();
    const metric = metrics[name] ??= { count: 0, errors: 0, inclusiveNs: 0 };
    metric.count++;
    let value;
    try { value = invoke(); }
    catch (error) { metric.errors++; throw error; }
    finally {
      const elapsed = process.hrtime.bigint() - start;
      metric.inclusiveNs += Number(elapsed);
      if (outer) insideNs += elapsed;
      depth--;
    }
    observe(name, args, value);
    return value;
  }
  return new Proxy(fn, {
    apply(target, receiver, args) { return timed(() => Reflect.apply(target, receiver, args), args); },
    construct(target, args, newTarget) { return timed(() => Reflect.construct(target, args, newTarget), args); }
  });
}
process.once('exit', exitCode => {
  const stateErrors = [];
  for (const name of ['exportSheet', 'exportClassMap', 'exportFileMap', 'exportCanonicalMap']) {
    try { states[name + 'AtExit'] = hash(encode(binding[name]())); }
    catch (error) { stateErrors.push({ name, error: String(error) }); }
  }
  const lifetimeMs = Number(process.hrtime.bigint() - started) / 1e6;
  fs.writeFileSync(process.env.W72_REAL_PREFIX + '-api-' + process.pid + '-' + threadId + '-' + token + '.json', JSON.stringify({ pid: process.pid, ppid: process.ppid, threadId, node: process.version, execPath: process.execPath, binding: process.env.W72_BINDING, initializationMs, exitCode, lifetimeMs, insideMs: Number(insideNs) / 1e6, insideShare: Number(insideNs) / 1e6 / lifetimeMs, observationMs: Number(observationNs) / 1e6, metrics, observations, observationErrors, states, stateErrors }));
});
${names.map((name) => `exports.${name} = wrap(${JSON.stringify(name)});`).join('\n')}
`
}
