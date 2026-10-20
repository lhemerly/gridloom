import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseEdit, displayValue } from '../src/utils/values.ts';

test('edits retain tagged API values and numeric precision', () => {
  assert.deepEqual(parseEdit('123456789.123456789', { type: 'Decimal', value: '1' }),
    { type: 'Decimal', value: '123456789.123456789' });
  assert.deepEqual(parseEdit('001', { type: 'Text', value: 'A' }), { type: 'Text', value: '001' });
  assert.deepEqual(parseEdit('2026-10-20', { type: 'Date', value: '2026-10-19' }), { type: 'Date', value: '2026-10-20' });
  assert.deepEqual(parseEdit('NULL', { type: 'Integer', value: 5 }), { type: 'Null' });
});
test('tagged values render as cell text rather than object strings', () => {
  assert.equal(displayValue({ type: 'Text', value: 'A' }), 'A');
  assert.equal(displayValue({ type: 'Decimal', value: '12.50' }), '12.50');
  assert.equal(displayValue({ type: 'Null' }), 'NULL');
});
