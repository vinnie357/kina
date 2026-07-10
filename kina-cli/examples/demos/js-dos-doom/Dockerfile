# Dockerfile for js-dos Doom — arm64 nginx image serving Doom shareware
#
# Builds a minimal arm64 nginx image with:
#   - js-dos runtime (wasm + JS)
#   - Doom shareware WAD (DOOM1.WAD)
#
# Build:
#   container build -t doom-game:arm64 .
#
# The js-dos assets are fetched from the official js-dos CDN at build time.
# No runtime network access is needed; everything is baked into the image.

FROM --platform=linux/arm64 nginx:1.27-alpine

# Install curl for asset download during build
RUN apk add --no-cache curl

# Download js-dos v8 runtime assets
# js-dos v8 is a WebAssembly-based DOS emulator that works on all browsers
WORKDIR /usr/share/nginx/html

# Download the Doom shareware WAD (freely distributable)
RUN curl -fsSL -o DOOM1.WAD \
    "https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad"

# Download js-dos v8 distribution (wasm + js bundle)
RUN curl -fsSL -o js-dos.js \
    "https://cdn.dos.zone/original/2X/3/37bf03cb498d31f8c30e875ec59f0de4f97e5c6d.js" \
    || curl -fsSL -o js-dos.js \
    "https://js.dos.zone/releases/v8/bundle.js"

# Create the launcher HTML
RUN cat > index.html <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Doom (js-dos) on Kubernetes</title>
  <style>
    body { margin: 0; background: #000; display: flex; justify-content: center; align-items: center; height: 100vh; }
    #dos { width: 640px; height: 400px; }
  </style>
</head>
<body>
  <div id="dos"></div>
  <script src="/js-dos.js"></script>
  <script>
    Dos(document.getElementById("dos"), {
      url: "/DOOM1.WAD",
      gameVersion: "doom"
    });
  </script>
</body>
</html>
EOF

# Remove the curl tool — not needed at runtime
RUN apk del curl

EXPOSE 80
