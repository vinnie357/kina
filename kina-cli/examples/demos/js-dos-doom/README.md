# js-dos Doom Demo

Runs the original Doom (shareware) in-browser via [js-dos](https://js.dos.zone/),
served as static files from an nginx container. Access via any browser pointed
at the node IP.

## arm64 Build Required

The js-dos distribution packages an amd64 WebAssembly/asm.js Doom binary, but
the container serving those assets is just nginx — and we need an arm64 nginx
container image. There is no pre-built public arm64 image for this demo;
you must build one.

### Build Steps

```bash
# 1. Build the arm64 image inside the cluster's node VM, or use container build
#    on your Mac (Apple Container uses native arm64 by default):
cd kina-cli/examples/demos/js-dos-doom/
container build -t doom-game:arm64 .

# 2. Push to your in-cluster registry (kina-42 tracks this addon):
#    Once kina install registry is available:
container image tag doom-game:arm64 <REGISTRY_IP>:5000/doom-game:arm64
container image push <REGISTRY_IP>:5000/doom-game:arm64

# 3. Then apply with the registry address:
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')
sed "s|REGISTRY/<name>:arm64|<REGISTRY_IP>:5000/doom-game:arm64|g; s/<NODE_IP>/$NODE_IP/g" \
    gatewayapi.yaml | kubectl apply -f -
```

Until kina-42 lands, you can load the image directly into the node container:

```bash
# Save and load into the node
container image save doom-game:arm64 | \
  container exec -i <arena-control-plane> ctr images import -
```

## Prerequisites

- Built arm64 image (see above)
- Either `kina install nginx-gateway-fabric` or `kina install nginx-ingress`
- `NODE_IP` and `REGISTRY_IP` from your cluster

## Deploy

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

# Gateway API variant
sed "s|REGISTRY/doom-game:arm64|<REGISTRY_IP>:5000/doom-game:arm64|g; \
     s/<NODE_IP>/$NODE_IP/g" gatewayapi.yaml | kubectl apply -f -

# nginx-ingress variant
sed "s|REGISTRY/doom-game:arm64|<REGISTRY_IP>:5000/doom-game:arm64|g; \
     s/<NODE_IP>/$NODE_IP/g" ingress.yaml | kubectl apply -f -
```

## Access

Open a browser on your Mac and navigate to:

```
http://doom-game.<NODE_IP>.nip.io
```

Click in the js-dos window and press Enter to start the game.

## Teardown

```bash
kubectl delete namespace doom-game
```

## Notes

- The js-dos assets (Doom shareware WAD + js-dos runtime) are baked into the
  image at build time from the js-dos CDN. No runtime network access needed.
- js-dos runs entirely in the browser via WebAssembly. The container is just
  a static file server.
- The Doom shareware WAD (`DOOM1.WAD`) is freely distributable.
