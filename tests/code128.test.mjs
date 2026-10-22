import test from 'node:test';
import assert from 'node:assert/strict';
import { code128Bars, code128Symbols, code128Svg } from '../src/lib/code128.ts';

test('Code 128 B includes start, checksum and stop symbols', () => {
  assert.deepEqual(code128Symbols('A'), [104, 33, 34, 106]);
  const bars = code128Bars('A');
  assert.equal(bars.reduce((sum, bar) => sum + bar.width, 0), 46);
  assert.deepEqual(bars.slice(0, 6).map(bar => bar.width), [2, 1, 1, 2, 1, 4]);
  assert.throws(() => code128Symbols('ñ'), /ASCII/);
  assert.match(code128Svg('A&B'), /A&amp;B/);
});
