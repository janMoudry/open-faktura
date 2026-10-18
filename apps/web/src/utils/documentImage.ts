/** Read bounded raster dimensions before asking the browser to allocate pixels. */
export function documentImageDimensions(bytes: Uint8Array): {
  width: number;
  height: number;
} {
  let width = 0,
    height = 0;
  if (
    bytes.length >= 24 &&
    bytes[0] === 137 &&
    String.fromCharCode(...bytes.subarray(12, 16)) === "IHDR"
  ) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    width = view.getUint32(16);
    height = view.getUint32(20);
  } else if (bytes[0] === 255 && bytes[1] === 216) {
    let offset = 2;
    while (offset + 4 <= bytes.length) {
      if (bytes[offset] !== 255) throw new Error("image marker");
      while (bytes[offset] === 255) offset++;
      const marker = bytes[offset++];
      if (marker === 217 || marker === 218) break;
      if (marker === 1 || (marker >= 208 && marker <= 215)) continue;
      const length = bytes[offset] * 256 + bytes[offset + 1];
      if (length < 2 || offset + length > bytes.length)
        throw new Error("image segment");
      if (
        [
          192, 193, 194, 195, 197, 198, 199, 201, 202, 203, 205, 206, 207,
        ].includes(marker) &&
        length >= 8
      ) {
        height = bytes[offset + 3] * 256 + bytes[offset + 4];
        width = bytes[offset + 5] * 256 + bytes[offset + 6];
        break;
      }
      offset += length;
    }
  }
  if (
    !width ||
    !height ||
    width > 4096 ||
    height > 4096 ||
    width * height > 16_000_000
  )
    throw new Error("image dimensions");
  return { width, height };
}
