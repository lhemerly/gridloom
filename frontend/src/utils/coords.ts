export function colIndexToName(col: number): string {
  let name = '';
  let curr = col;
  while (curr >= 0) {
    name = String.fromCharCode((curr % 26) + 65) + name;
    curr = Math.floor(curr / 26) - 1;
  }
  return name;
}

export function colNameToIndex(name: string): number {
  let col = 0;
  for (let i = 0; i < name.length; i++) {
    col = col * 26 + (name.charCodeAt(i) - 64);
  }
  return col - 1;
}

export function coordToA1(row: number, col: number): string {
  return `${colIndexToName(col)}${row + 1}`;
}

export function a1ToCoord(a1: string): { row: number; col: number } | null {
  const match = a1.trim().toUpperCase().match(/^([A-Z]+)(\d+)$/);
  if (!match) return null;
  const col = colNameToIndex(match[1]);
  const row = parseInt(match[2], 10) - 1;
  return { row, col };
}
