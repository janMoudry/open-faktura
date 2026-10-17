// The isolated preview shares the development API, whose trusted UI is :5173.
// Translate only the preview's own Origin; never bless a foreign or absent one.
export function developmentApiOrigin(origin, host) {
  if (
    (host === "127.0.0.1:5174" || host === "localhost:5174") &&
    origin === `http://${host}`
  )
    return "http://127.0.0.1:5173";
  return origin;
}
