import type { CellValue } from '../types';

export function parseEdit(input: string, original: CellValue): CellValue {
  // Text keys, codes and labels must retain their text identity (including leading zeros).
  if (original.type === 'Text' || original.type === 'EmptyText') return { type: 'Text', value: input };
  if (input.trim().toUpperCase() === 'NULL') return { type: 'Null' };
  if (original.type === 'Date' || original.type === 'Timestamp') return { type: original.type, value: input };
  if (/^(true|false)$/i.test(input.trim())) return { type: 'Boolean', value: input.trim().toLowerCase() === 'true' };
  if (/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/.test(input.trim())) {
    return { type: 'Decimal', value: input.trim() };
  }
  return { type: 'Text', value: input };
}

export function displayValue(value: CellValue): string {
  switch (value.type) {
    case 'Null': return 'NULL';
    case 'Blank': case 'EmptyText': return '';
    case 'Error': return JSON.stringify(value.value);
    case 'Array': return value.value.map(r => r.map(displayValue).join(', ')).join('; ');
    default: return String(value.value);
  }
}
