export interface Series {
  codes: string[];
  firstYear: number;
  years: number;
  values: Float32Array;
}

const HEADER = 12;

export function decodeSeries(buffer: ArrayBuffer): Series {
  if (buffer.byteLength < HEADER) throw new Error("series: truncated header");
  const view = new DataView(buffer);
  for (let i = 0; i < 4; i++) {
    if (view.getUint8(i) !== "MSR1".charCodeAt(i)) throw new Error("series: bad magic");
  }
  const count = view.getUint16(4, true);
  const firstYear = view.getUint16(6, true);
  const years = view.getUint16(8, true);
  const data = Math.ceil((HEADER + 3 * count) / 4) * 4;
  const size = data + 4 * count * years;
  if (buffer.byteLength < size) throw new Error("series: truncated body");
  const codes: string[] = [];
  for (let i = 0; i < count; i++) {
    const at = HEADER + 3 * i;
    codes.push(
      String.fromCharCode(view.getUint8(at), view.getUint8(at + 1), view.getUint8(at + 2)),
    );
  }
  const values = new Float32Array(count * years);
  for (let i = 0; i < values.length; i++) values[i] = view.getFloat32(data + 4 * i, true);
  return { codes, firstYear, years, values };
}
