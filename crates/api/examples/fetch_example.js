// Browser / Tauri front-end example: send Schnorr proof (base64 canonical bytes)

// helper: base64 encode Uint8Array
function bytesToBase64(bytes) {
  let binary = '';
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

// helper: base64 decode to Uint8Array
function base64ToBytes(b64) {
  const bin = atob(b64);
  const len = bin.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

// Example: you already obtained the canonical bytes (e.g., from wasm or native Tauri API)
// rBytes, sBytes, pkShareBytes, messageBytes are Uint8Array
async function sendProof(rBytes, sBytes, pkShareBytes, messageBytes, nonce, ts) {
  const payload = {
    message: bytesToBase64(messageBytes),
    proof: {
      R: bytesToBase64(rBytes),
      s: bytesToBase64(sBytes),
      pk_share: bytesToBase64(pkShareBytes),
      nonce: nonce,
      ts: ts,
    }
  };

  const res = await fetch('https://localhost:8443/proof/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });

  const j = await res.json();
  console.log('verify result', j);
  return j;
}

// Usage notes:
// - In Tauri, you can either call the same fetch (if using remote server) or call a Rust command
//   that returns the base64 bytes (and then call the same `sendProof` with those bytes).
// - If using wasm, export an API that returns base64 strings already (or Uint8Array) so the
//   front-end doesn't have to re-serialize.
