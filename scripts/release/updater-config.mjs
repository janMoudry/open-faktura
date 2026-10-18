import { createPublicKey, verify, createHash } from "node:crypto";
export function updaterConfig(env = process.env) {
  const endpoint = (env.OPEN_FAKTURA_UPDATER_ENDPOINT ?? "").trim(),
    publicKey = (env.OPEN_FAKTURA_UPDATER_PUBLIC_KEY ?? "").trim();
  if (!endpoint && !publicKey) return null;
  if (!endpoint || !publicKey)
    throw new Error("Both updater endpoint and public key are required");
  const url = new URL(endpoint);
  if (url.protocol !== "https:" || url.username || url.password || url.hash)
    throw new Error(
      "Updater endpoint must be public HTTPS without credentials or fragment",
    );
  decodeKey(publicKey);
  return { endpoint, publicKey };
}
function decode64(text) {
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(text.trim()))
    throw new Error("Invalid updater base64");
  return Buffer.from(text.trim(), "base64");
}
function decodeKey(encoded) {
  const lines = decode64(encoded).toString("utf8").trim().split(/\r?\n/);
  if (lines.length !== 2 || !lines[0].startsWith("untrusted comment:"))
    throw new Error("Invalid updater public key");
  const packet = decode64(lines[1]);
  if (packet.length !== 42 || packet.subarray(0, 2).toString() !== "Ed")
    throw new Error("Invalid updater public key");
  return {
    packet,
    key: createPublicKey({
      key: Buffer.concat([
        Buffer.from("302a300506032b6570032100", "hex"),
        packet.subarray(10),
      ]),
      format: "der",
      type: "spki",
    }),
  };
}
/** Minisign format verified with Node's Ed25519 and BLAKE2b primitives.
 * Layout/global signature follows pinned minisign-verify 0.3.0; no private keys. */
export function verifyUpdaterArtifact(
  bytes,
  encodedSignature,
  encodedKey,
  version,
) {
  const { packet: pub, key } = decodeKey(encodedKey);
  const lines = decode64(encodedSignature)
    .toString("utf8")
    .trim()
    .split(/\r?\n/);
  if (
    lines.length !== 4 ||
    !lines[0].startsWith("untrusted comment:") ||
    !lines[2].startsWith("trusted comment: ")
  )
    throw new Error("Invalid updater signature");
  const packet = decode64(lines[1]),
    global = decode64(lines[3]);
  if (
    packet.length !== 74 ||
    global.length !== 64 ||
    !packet.subarray(2, 10).equals(pub.subarray(2, 10))
  )
    throw new Error("Updater key mismatch");
  const algorithm = packet.subarray(0, 2).toString();
  const payload =
    algorithm === "ED"
      ? createHash("blake2b512").update(bytes).digest()
      : algorithm === "Ed"
        ? bytes
        : null;
  if (!payload || !verify(null, payload, key, packet.subarray(10)))
    throw new Error("Updater artifact signature mismatch");
  const comment = lines[2].slice("trusted comment: ".length);
  if (
    !verify(
      null,
      Buffer.concat([packet.subarray(10), Buffer.from(comment)]),
      key,
      global,
    )
  )
    throw new Error("Updater trusted comment signature mismatch");
  const signed = comment
    .split("\t")
    .find((v) => v.startsWith("version:"))
    ?.slice(8);
  if (!signed || signed.replace(/^v/, "") !== version.replace(/^v/, ""))
    throw new Error("Updater signed version mismatch");
  return true;
}
